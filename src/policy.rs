use crate::hooks::Hook;
use crate::types::{HookResult, ToolCall};
use async_trait::async_trait;
use std::path::Path;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    Approve,
    Deny,
    AskUser,
}

#[derive(Clone)]
pub struct Policy {
    pub tool: String,
    pub decision: Decision,
    pub when: Option<Arc<dyn Fn(&ToolCall) -> bool + Send + Sync>>,
    pub ask_user: Option<Arc<dyn Fn(&ToolCall) -> bool + Send + Sync>>,
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
}

pub fn allow(tool: &str) -> Policy {
    Policy::new(
        tool.to_string(),
        Decision::Approve,
        None,
        None,
        String::new(),
    )
}

pub fn deny(tool: &str) -> Policy {
    Policy::new(tool.to_string(), Decision::Deny, None, None, String::new())
}

pub fn ask_user(tool: &str, handler: impl Fn(&ToolCall) -> bool + Send + Sync + 'static) -> Policy {
    Policy::new(
        tool.to_string(),
        Decision::AskUser,
        None,
        Some(Arc::new(handler)),
        String::new(),
    )
}

pub fn allow_all() -> Policy {
    Policy::new(
        "*".to_string(),
        Decision::Approve,
        None,
        None,
        "allow_all".to_string(),
    )
}

pub fn deny_all() -> Policy {
    Policy::new(
        "*".to_string(),
        Decision::Deny,
        None,
        None,
        "deny_all".to_string(),
    )
}

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

#[derive(Debug)]
pub struct PolicyEnforcer {
    policies: Vec<Policy>,
}

impl PolicyEnforcer {
    pub const fn new(policies: Vec<Policy>) -> Self {
        Self { policies }
    }
}

#[async_trait]
impl Hook for PolicyEnforcer {
    async fn pre_tool_call(&self, tool_call: &ToolCall) -> Result<HookResult, anyhow::Error> {
        // Enforce specificity and precedence rules:
        // Priority bucket levels:
        // 0: Specific Deny
        // 1: Specific Ask
        // 2: Specific Allow
        // 3: Wildcard Deny
        // 4: Wildcard Ask
        // 5: Wildcard Allow

        let mut matched_policy: Option<&Policy> = None;
        let mut matched_level = 6;

        for p in &self.policies {
            let matches_tool = p.tool == "*" || p.tool == tool_call.name;
            if !matches_tool {
                continue;
            }

            if p.when.as_ref().is_some_and(|when_fn| !when_fn(tool_call)) {
                continue;
            }

            let level = match (p.tool == "*", p.decision) {
                (false, Decision::Deny) => 0,
                (false, Decision::AskUser) => 1,
                (false, Decision::Approve) => 2,
                (true, Decision::Deny) => 3,
                (true, Decision::AskUser) => 4,
                (true, Decision::Approve) => 5,
            };

            if level < matched_level {
                matched_level = level;
                matched_policy = Some(p);
            }
        }

        if let Some(p) = matched_policy {
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
                        if handler(tool_call) {
                            return Ok(HookResult {
                                allow: true,
                                message: String::new(),
                            });
                        }
                        return Ok(HookResult {
                            allow: false,
                            message: format!(
                                "User denied tool '{}' (policy '{}').",
                                tool_call.name, label
                            ),
                        });
                    }
                }
            }
        }

        // Default open if no policies match
        Ok(HookResult {
            allow: true,
            message: String::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use serde_json::Value;

    #[tokio::test]
    async fn test_allow_deny_rules() {
        let policies = vec![deny_all(), allow("VIEW_FILE")];
        let enforcer = PolicyEnforcer::new(policies);

        // VIEW_FILE should be approved (specific allow wins over wildcard deny)
        let tc1 = ToolCall {
            id: "1".to_string(),
            name: "VIEW_FILE".to_string(),
            args: Value::Null,
            canonical_path: None,
        };
        let res1 = enforcer.pre_tool_call(&tc1).await.unwrap();
        assert!(res1.allow);

        // EDIT_FILE should be denied (wildcard deny)
        let tc2 = ToolCall {
            id: "2".to_string(),
            name: "EDIT_FILE".to_string(),
            args: Value::Null,
            canonical_path: None,
        };
        let res2 = enforcer.pre_tool_call(&tc2).await.unwrap();
        assert!(!res2.allow);
    }

    #[tokio::test]
    async fn test_workspace_only() {
        let policies = workspace_only(vec!["/allowed/workspace".to_string()]);
        let enforcer = PolicyEnforcer::new(policies);

        // CREATE_FILE inside workspace should be allowed (doesn't trigger the deny policy)
        let tc1 = ToolCall {
            id: "1".to_string(),
            name: "CREATE_FILE".to_string(),
            args: Value::Null,
            canonical_path: Some("/allowed/workspace/subdir/file.rs".to_string()),
        };
        let res1 = enforcer.pre_tool_call(&tc1).await.unwrap();
        assert!(res1.allow);

        // CREATE_FILE outside workspace should be denied
        let tc2 = ToolCall {
            id: "2".to_string(),
            name: "CREATE_FILE".to_string(),
            args: Value::Null,
            canonical_path: Some("/forbidden/path/file.rs".to_string()),
        };
        let res2 = enforcer.pre_tool_call(&tc2).await.unwrap();
        assert!(!res2.allow);
    }
}
