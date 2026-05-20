//! Safety policies and middleware for tool execution control.
//!
//! This module defines the safety policies that can be configured on an [`crate::agent::Agent`]
//! to restrict tool invocation permissions, establish sandbox/workspace boundaries,
//! or require explicit user confirmation.

use crate::hooks::Hook;
use crate::types::{HookResult, ToolCall};
use async_trait::async_trait;
use std::path::Path;
use std::sync::Arc;

/// Represents the safety enforcement action to take when a tool invocation is intercepted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    /// Allow execution of the tool automatically.
    Approve,
    /// Block execution and return an access denied error.
    Deny,
    /// Ask the user for confirmation before executing the tool.
    AskUser,
}

/// A policy configuration mapping a tool name to a safety [`Decision`].
///
/// Wildcard (`*`) rules are supported to act as a fallback, which are overridden
/// by specific tool rules according to a precedence bucketed hierarchy.
#[derive(Clone)]
pub struct Policy {
    /// The exact tool name, or `*` for a wildcard matching any tool.
    pub tool: String,
    /// The enforcement decision to apply when the tool matches.
    pub decision: Decision,
    /// An optional filter function that decides if the policy applies based on the tool parameters.
    pub when: Option<Arc<dyn Fn(&ToolCall) -> bool + Send + Sync>>,
    /// An optional callback deciding whether to prompt the user for confirmation.
    pub ask_user: Option<Arc<dyn Fn(&ToolCall) -> bool + Send + Sync>>,
    /// A descriptive name for the safety policy.
    pub name: String,
}

impl std::fmt::Debug for Policy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Policy")
            .field("tool", &self.tool)
            .field("decision", &self.decision)
            .field("when_is_some", &self.when.is_some())
            .field("ask_user_is_some", &self.ask_user.is_some())
            .field("name", &self.name)
            .finish()
    }
}

impl Policy {
    /// Creates a new policy rule.
    pub fn new(
        tool: String,
        decision: Decision,
        when: Option<Arc<dyn Fn(&ToolCall) -> bool + Send + Sync>>,
        ask_user: Option<Arc<dyn Fn(&ToolCall) -> bool + Send + Sync>>,
        name: String,
    ) -> Self {
        Self {
            tool,
            decision,
            when,
            ask_user,
            name,
        }
    }

    /// Adds a conditional filter function to the policy.
    pub fn when(mut self, when_fn: impl Fn(&ToolCall) -> bool + Send + Sync + 'static) -> Self {
        self.when = Some(Arc::new(when_fn));
        self
    }

    /// Customizes the policy's debug/tracing name.
    pub fn with_name(mut self, name: &str) -> Self {
        self.name = name.to_string();
        self
    }
}

/// Helper constructor to approve a specific tool invocation unconditionally.
pub fn allow(tool: &str) -> Policy {
    Policy::new(
        tool.to_string(),
        Decision::Approve,
        None,
        None,
        String::new(),
    )
}

/// Helper constructor to deny a specific tool invocation unconditionally.
pub fn deny(tool: &str) -> Policy {
    Policy::new(tool.to_string(), Decision::Deny, None, None, String::new())
}

/// Helper constructor to require user confirmation for a specific tool.
pub fn ask_user(tool: &str, handler: impl Fn(&ToolCall) -> bool + Send + Sync + 'static) -> Policy {
    Policy::new(
        tool.to_string(),
        Decision::AskUser,
        None,
        Some(Arc::new(handler)),
        String::new(),
    )
}

/// Helper constructor to approve all tool calls as a wildcard fallback.
pub fn allow_all() -> Policy {
    Policy::new(
        "*".to_string(),
        Decision::Approve,
        None,
        None,
        "allow_all".to_string(),
    )
}

/// Helper constructor to deny all tool calls as a wildcard fallback.
pub fn deny_all() -> Policy {
    Policy::new(
        "*".to_string(),
        Decision::Deny,
        None,
        None,
        "deny_all".to_string(),
    )
}

/// Creates a standard set of policies that requires user approval for commands or denies them.
pub fn confirm_run_command(
    handler: Option<Arc<dyn Fn(&ToolCall) -> bool + Send + Sync>>,
) -> Vec<Policy> {
    handler.map_or_else(
        || {
            vec![
                Policy::new(
                    "RUN_COMMAND".to_string(),
                    Decision::Deny,
                    None,
                    None,
                    "confirm_run_command".to_string(),
                ),
                allow_all(),
            ]
        },
        |h| {
            vec![
                Policy::new(
                    "RUN_COMMAND".to_string(),
                    Decision::AskUser,
                    None,
                    Some(h),
                    "confirm_run_command".to_string(),
                ),
                allow_all(),
            ]
        },
    )
}

/// Creates a set of policies restricting file system tools to specified workspace directory paths.
pub fn workspace_only(workspaces: Vec<String>) -> Vec<Policy> {
    let file_tools = vec![
        "CREATE_FILE",
        "EDIT_FILE",
        "VIEW_FILE",
        "LIST_DIR",
        "SEARCH_DIR",
    ];

    let is_outside_workspace = move |tc: &ToolCall| -> bool {
        let path_str = tc.canonical_path.as_deref().unwrap_or("");
        if path_str.is_empty() {
            return false;
        }
        let target_path = Path::new(path_str);
        if !target_path.is_absolute() {
            return true;
        }
        for ws in &workspaces {
            let ws_path = Path::new(ws);
            if ws_path.is_absolute() && target_path.starts_with(ws_path) {
                return false;
            }
        }
        true
    };

    let when_fn = Arc::new(is_outside_workspace);

    file_tools
        .into_iter()
        .map(|tool| {
            Policy::new(
                tool.to_string(),
                Decision::Deny,
                Some(when_fn.clone()),
                None,
                "workspace_only".to_string(),
            )
        })
        .collect()
}

/// Validates and compiles a list of [`Policy`] items into a [`PolicyEnforcer`].
///
/// # Errors
///
/// Returns an error if any `AskUser` policy is missing a confirmation handler callback.
pub fn enforce(policies: Vec<Policy>) -> Result<PolicyEnforcer, anyhow::Error> {
    for p in &policies {
        if p.decision == Decision::AskUser && p.ask_user.is_none() {
            return Err(anyhow::anyhow!(
                "ASK_USER policy '{}' is missing an ask_user handler. Provide one via policy.ask_user(tool, handler).",
                if p.name.is_empty() { &p.tool } else { &p.name }
            ));
        }
    }
    Ok(PolicyEnforcer::new(policies))
}

/// Safety policy middleware enforcer implemented as a lifecycle [`Hook`].
pub struct PolicyEnforcer {
    buckets: [Vec<Policy>; 6],
}

impl PolicyEnforcer {
    /// Compiles policies into prioritized buckets for performance and precedence.
    pub fn new(policies: Vec<Policy>) -> Self {
        let mut buckets = [
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        ];
        for p in policies {
            let level = match (p.tool == "*", p.decision) {
                (false, Decision::Deny) => 0,
                (false, Decision::AskUser) => 1,
                (false, Decision::Approve) => 2,
                (true, Decision::Deny) => 3,
                (true, Decision::AskUser) => 4,
                (true, Decision::Approve) => 5,
            };
            buckets[level].push(p);
        }
        Self { buckets }
    }
}

impl std::fmt::Debug for PolicyEnforcer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PolicyEnforcer")
            .field("buckets", &self.buckets)
            .finish()
    }
}

#[async_trait]
impl Hook for PolicyEnforcer {
    async fn pre_tool_call(&self, tool_call: &ToolCall) -> Result<HookResult, anyhow::Error> {
        for bucket in &self.buckets {
            for p in bucket {
                let matches_tool = p.tool == "*" || p.tool == tool_call.name;
                if !matches_tool {
                    continue;
                }

                if let Some(ref when_fn) = p.when {
                    let when_fn = when_fn.clone();
                    let tc = tool_call.clone();
                    let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
                        when_fn(&tc)
                    }));
                    match res {
                        Ok(true) => {}
                        Ok(false) => continue,
                        Err(_) => {
                            let label = if p.name.is_empty() { &p.tool } else { &p.name };
                            return Ok(HookResult {
                                allow: false,
                                message: format!(
                                    "Policy evaluation failed for policy '{label}': predicate panicked."
                                ),
                            });
                        }
                    }
                }

                let label = if p.name.is_empty() { &p.tool } else { &p.name };
                match p.decision {
                    Decision::Deny => {
                        return Ok(HookResult {
                            allow: false,
                            message: format!("Denied by policy '{label}'."),
                        });
                    }
                    Decision::Approve => {
                        return Ok(HookResult {
                            allow: true,
                            message: String::new(),
                        });
                    }
                    Decision::AskUser => {
                        if let Some(ref handler) = p.ask_user {
                            let handler = handler.clone();
                            let tc = tool_call.clone();
                            let res =
                                std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
                                    handler(&tc)
                                }));
                            match res {
                                Ok(true) => {
                                    return Ok(HookResult {
                                        allow: true,
                                        message: String::new(),
                                    });
                                }
                                Ok(false) => {
                                    return Ok(HookResult {
                                        allow: false,
                                        message: format!(
                                            "User denied tool '{}' (policy '{}').",
                                            tool_call.name, label
                                        ),
                                    });
                                }
                                Err(_) => {
                                    return Ok(HookResult {
                                        allow: false,
                                        message: format!(
                                            "Policy evaluation failed for policy '{label}': handler panicked."
                                        ),
                                    });
                                }
                            }
                        }
                        return Err(anyhow::anyhow!(
                            "ASK_USER policy '{}' is missing an ask_user handler.",
                            label
                        ));
                    }
                }
            }
        }

        Ok(HookResult {
            allow: true,
            message: String::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::unnecessary_map_or
    )]
    use super::*;
    use serde_json::json;

    fn make_tool_call(name: &str, args: serde_json::Value) -> ToolCall {
        let mut canonical_path = None;
        if let serde_json::Value::Object(ref map) = args {
            for path_key in &["path", "file_path", "TargetFile", "directory_path"] {
                if let Some(serde_json::Value::String(val)) = map.get(*path_key) {
                    canonical_path = Some(val.clone());
                    break;
                }
            }
        }
        ToolCall {
            id: "call_123".to_string(),
            name: name.to_string(),
            args,
            canonical_path,
        }
    }

    #[test]
    fn test_allow_creates_approve_policy() {
        let p = allow("read_file").with_name("allow-read");
        assert_eq!(p.tool, "read_file");
        assert_eq!(p.decision, Decision::Approve);
        assert!(p.when.is_none());
        assert!(p.ask_user.is_none());
        assert_eq!(p.name, "allow-read");
    }

    #[test]
    fn test_deny_creates_deny_policy() {
        let p = deny("run_command").with_name("block-cmd");
        assert_eq!(p.tool, "run_command");
        assert_eq!(p.decision, Decision::Deny);
        assert_eq!(p.name, "block-cmd");
    }

    #[test]
    fn test_ask_user_creates_ask_user_policy() {
        let p = ask_user("run_command", |_| true).with_name("confirm-cmd");
        assert_eq!(p.decision, Decision::AskUser);
        assert!(p.ask_user.is_some());
        assert_eq!(p.name, "confirm-cmd");
    }

    #[test]
    fn test_deny_with_predicate() {
        let p = deny("run_command").when(|args| {
            args.args
                .get("CommandLine")
                .and_then(|v| v.as_str())
                .map_or(false, |s| s.contains("rm"))
        });
        assert!(p.when.is_some());
    }

    #[test]
    fn test_allow_all_creates_wildcard_approve() {
        let p = allow_all();
        assert_eq!(p.tool, "*");
        assert_eq!(p.decision, Decision::Approve);
        assert_eq!(p.name, "allow_all");
    }

    #[test]
    fn test_deny_all_creates_wildcard_deny() {
        let p = deny_all();
        assert_eq!(p.tool, "*");
        assert_eq!(p.decision, Decision::Deny);
        assert_eq!(p.name, "deny_all");
    }

    #[test]
    fn test_enforce_rejects_ask_user_without_handler() {
        let bad_policy = Policy::new(
            "run_command".to_string(),
            Decision::AskUser,
            None,
            None,
            "oops".to_string(),
        );
        let res = enforce(vec![bad_policy]);
        assert!(res.is_err());
        let err_msg = res.err().unwrap().to_string();
        assert!(err_msg.contains("oops"));
        assert!(err_msg.contains("missing an ask_user handler"));
    }

    #[tokio::test]
    async fn test_specific_deny_overrides_wildcard_allow() {
        let enforcer = enforce(vec![allow_all(), deny("dangerous_tool")]).unwrap();
        let res = enforcer
            .pre_tool_call(&make_tool_call("dangerous_tool", json!({})))
            .await
            .unwrap();
        assert!(!res.allow);
    }

    #[tokio::test]
    async fn test_specific_deny_overrides_specific_allow() {
        let enforcer = enforce(vec![allow("run_command"), deny("run_command")]).unwrap();
        let res = enforcer
            .pre_tool_call(&make_tool_call("run_command", json!({})))
            .await
            .unwrap();
        assert!(!res.allow);
    }

    #[tokio::test]
    async fn test_specific_ask_overrides_wildcard_deny() {
        let enforcer = enforce(vec![deny_all(), ask_user("run_command", |_| true)]).unwrap();
        let res = enforcer
            .pre_tool_call(&make_tool_call("run_command", json!({})))
            .await
            .unwrap();
        assert!(res.allow);
    }

    #[tokio::test]
    async fn test_specific_allow_overrides_wildcard_deny() {
        let enforcer = enforce(vec![deny_all(), allow("read_file")]).unwrap();

        let res = enforcer
            .pre_tool_call(&make_tool_call("read_file", json!({})))
            .await
            .unwrap();
        assert!(res.allow);

        let res = enforcer
            .pre_tool_call(&make_tool_call("run_command", json!({})))
            .await
            .unwrap();
        assert!(!res.allow);
    }

    #[tokio::test]
    async fn test_wildcard_deny_blocks_unmatched_tools() {
        let enforcer = enforce(vec![deny_all()]).unwrap();
        let res = enforcer
            .pre_tool_call(&make_tool_call("anything", json!({})))
            .await
            .unwrap();
        assert!(!res.allow);
    }

    #[tokio::test]
    async fn test_wildcard_ask_user() {
        let enforcer = enforce(vec![ask_user("*", |_| false)]).unwrap();
        let res = enforcer
            .pre_tool_call(&make_tool_call("any_tool", json!({})))
            .await
            .unwrap();
        assert!(!res.allow);
    }

    #[tokio::test]
    async fn test_wildcard_allow() {
        let enforcer = enforce(vec![allow_all()]).unwrap();
        let res = enforcer
            .pre_tool_call(&make_tool_call("any_tool", json!({})))
            .await
            .unwrap();
        assert!(res.allow);
    }

    #[tokio::test]
    async fn test_first_match_wins_within_deny_group() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static CALL_COUNT: AtomicUsize = AtomicUsize::new(0);

        let enforcer = enforce(vec![
            deny("run_command")
                .when(|_| {
                    CALL_COUNT.fetch_add(1, Ordering::SeqCst);
                    true
                })
                .with_name("first"),
            deny("run_command")
                .when(|_| {
                    CALL_COUNT.fetch_add(1, Ordering::SeqCst);
                    true
                })
                .with_name("second"),
        ])
        .unwrap();

        let res = enforcer
            .pre_tool_call(&make_tool_call("run_command", json!({})))
            .await
            .unwrap();
        assert!(!res.allow);
        assert_eq!(CALL_COUNT.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_first_match_wins_within_allow_group() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static CALL_COUNT: AtomicUsize = AtomicUsize::new(0);

        let enforcer = enforce(vec![
            allow("read_file").when(|_| {
                CALL_COUNT.fetch_add(1, Ordering::SeqCst);
                true
            }),
            allow("read_file").when(|_| {
                CALL_COUNT.fetch_add(1, Ordering::SeqCst);
                true
            }),
        ])
        .unwrap();

        let res = enforcer
            .pre_tool_call(&make_tool_call("read_file", json!({})))
            .await
            .unwrap();
        assert!(res.allow);
        assert_eq!(CALL_COUNT.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_skips_non_matching_predicate() {
        let enforcer = enforce(vec![
            deny("run_command").when(|_| false).with_name("skip-me"),
            deny("run_command").when(|_| true).with_name("catch-me"),
        ])
        .unwrap();

        let res = enforcer
            .pre_tool_call(&make_tool_call("run_command", json!({})))
            .await
            .unwrap();
        assert!(!res.allow);
        assert!(res.message.contains("catch-me"));
    }

    #[tokio::test]
    async fn test_predicate_exception_matches_fail_closed() {
        let enforcer = enforce(vec![
            deny("run_command")
                .when(|_| {
                    panic!("boom");
                })
                .with_name("broken"),
        ])
        .unwrap();

        let res = enforcer
            .pre_tool_call(&make_tool_call("run_command", json!({})))
            .await
            .unwrap();
        assert!(!res.allow);
        assert!(res.message.contains("broken"));
        assert!(res.message.contains("panicked"));
    }

    #[tokio::test]
    async fn test_handler_exception_denies() {
        let enforcer = enforce(vec![
            ask_user("run_command", |_| {
                panic!("handler broke");
            })
            .with_name("broken-ask"),
        ])
        .unwrap();

        let res = enforcer
            .pre_tool_call(&make_tool_call("run_command", json!({})))
            .await
            .unwrap();
        assert!(!res.allow);
        assert!(res.message.contains("broken-ask"));
        assert!(res.message.contains("handler panicked"));
    }

    #[tokio::test]
    async fn test_no_matching_policy_allows() {
        let enforcer = enforce(vec![deny("other_tool")]).unwrap();
        let res = enforcer
            .pre_tool_call(&make_tool_call("unrelated_tool", json!({})))
            .await
            .unwrap();
        assert!(res.allow);
    }

    #[tokio::test]
    async fn test_empty_policies_allows_all() {
        let enforcer = enforce(vec![]).unwrap();
        let res = enforcer
            .pre_tool_call(&make_tool_call("any_tool", json!({})))
            .await
            .unwrap();
        assert!(res.allow);
    }

    #[tokio::test]
    async fn test_workspace_only() {
        let policies = workspace_only(vec!["/allowed/workspace".to_string()]);
        let enforcer = enforce(policies).unwrap();

        let tc1 = make_tool_call(
            "VIEW_FILE",
            json!({"path": "/allowed/workspace/subdir/file.rs"}),
        );
        let res1 = enforcer.pre_tool_call(&tc1).await.unwrap();
        assert!(res1.allow);

        let tc2 = make_tool_call("VIEW_FILE", json!({"path": "/forbidden/path/file.rs"}));
        let res2 = enforcer.pre_tool_call(&tc2).await.unwrap();
        assert!(!res2.allow);
    }
}
