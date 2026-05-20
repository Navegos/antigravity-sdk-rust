//! Lifecycle event hooks for the agent execution loop.
//!
//! This module defines the [`Hook`] trait, which allows implementing custom observers and middlewares
//! to intercept session startup, pre/post tool invocations, execution errors, and user interactions.

use crate::types::{AskQuestionEntry, HookResult, QuestionHookResult, ToolCall, ToolResult};
use async_trait::async_trait;
use std::sync::Arc;

/// Trait representing an active interceptor of agent lifecycle events.
///
/// Implementors can register hooks via [`Agent::register_hook`](crate::agent::Agent::register_hook)
/// to audit tool invocations, log events, or restrict actions dynamically.
#[async_trait]
pub trait Hook: Send + Sync {
    /// Triggered when the agent establishes a connection and starts a session.
    async fn on_session_start(&self) -> Result<(), anyhow::Error> {
        Ok(())
    }
    /// Intercepts the start of a user turn before the LLM processes the prompt.
    /// Returns `allow: false` to halt execution.
    async fn pre_turn(&self) -> Result<HookResult, anyhow::Error> {
        Ok(HookResult {
            allow: true,
            message: String::new(),
        })
    }
    /// Intercepts a tool call immediately before it is executed by the runner.
    /// Returns `allow: false` to prevent execution.
    async fn pre_tool_call(&self, _tool_call: &ToolCall) -> Result<HookResult, anyhow::Error> {
        Ok(HookResult {
            allow: true,
            message: String::new(),
        })
    }
    /// Triggered after a tool successfully returns a result.
    async fn post_tool_call(&self, _result: &ToolResult) -> Result<(), anyhow::Error> {
        Ok(())
    }
    /// Triggered when a tool execution encounters an error.
    /// Allows fallback logic or customized error payloads.
    async fn on_tool_error(
        &self,
        error: &anyhow::Error,
    ) -> Result<(HookResult, Option<serde_json::Value>), anyhow::Error> {
        Ok((
            HookResult {
                allow: false,
                message: error.to_string(),
            },
            None,
        ))
    }
    /// Intercepts a prompt to ask the user clarifying questions.
    async fn on_interaction(
        &self,
        _questions: &[AskQuestionEntry],
    ) -> Result<Option<QuestionHookResult>, anyhow::Error> {
        Ok(None)
    }
}

/// Internal helper that manages a collection of registered [`Hook`]s and dispatches events sequentially.
#[derive(Clone, Default)]
pub struct HookRunner {
    hooks: Arc<tokio::sync::RwLock<Vec<Arc<dyn Hook>>>>,
}

impl std::fmt::Debug for HookRunner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HookRunner")
            .field("hooks_count", &self.hooks.try_read().map_or(0, |h| h.len()))
            .finish()
    }
}

impl HookRunner {
    /// Creates a new, empty `HookRunner`.
    pub fn new() -> Self {
        Self {
            hooks: Arc::new(tokio::sync::RwLock::new(Vec::new())),
        }
    }

    pub async fn register(&self, hook: Arc<dyn Hook>) {
        self.hooks.write().await.push(hook);
    }

    pub async fn dispatch_session_start(&self) -> Result<(), anyhow::Error> {
        let hooks = self.hooks.read().await.clone();
        for hook in &hooks {
            hook.on_session_start().await?;
        }
        Ok(())
    }

    pub async fn dispatch_pre_turn(&self) -> Result<HookResult, anyhow::Error> {
        let hooks = self.hooks.read().await.clone();
        for hook in &hooks {
            let res = hook.pre_turn().await?;
            if !res.allow {
                return Ok(res);
            }
        }
        Ok(HookResult {
            allow: true,
            message: String::new(),
        })
    }

    pub async fn dispatch_pre_tool_call(
        &self,
        tool_call: &ToolCall,
    ) -> Result<HookResult, anyhow::Error> {
        let hooks = self.hooks.read().await.clone();
        for hook in &hooks {
            let res = hook.pre_tool_call(tool_call).await?;
            if !res.allow {
                return Ok(res);
            }
        }
        Ok(HookResult {
            allow: true,
            message: String::new(),
        })
    }

    pub async fn dispatch_post_tool_call(&self, result: &ToolResult) -> Result<(), anyhow::Error> {
        let hooks = self.hooks.read().await.clone();
        for hook in &hooks {
            hook.post_tool_call(result).await?;
        }
        Ok(())
    }

    pub async fn dispatch_on_tool_error(
        &self,
        error: &anyhow::Error,
    ) -> Result<(HookResult, Option<serde_json::Value>), anyhow::Error> {
        let hooks = self.hooks.read().await.clone();
        for hook in &hooks {
            let (res, val) = hook.on_tool_error(error).await?;
            if res.allow {
                return Ok((res, val));
            }
        }
        Ok((
            HookResult {
                allow: false,
                message: error.to_string(),
            },
            None,
        ))
    }

    pub async fn dispatch_interaction(
        &self,
        questions: &[AskQuestionEntry],
    ) -> Result<Option<QuestionHookResult>, anyhow::Error> {
        let hooks = self.hooks.read().await.clone();
        for hook in &hooks {
            if let Some(res) = hook.on_interaction(questions).await? {
                return Ok(Some(res));
            }
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::field_reassign_with_default
    )]
    use super::*;
    use crate::types::{HookResult, QuestionHookResult, ToolCall, ToolResult};
    use std::sync::Mutex;

    struct TrackerHook {
        name: String,
        calls: Arc<Mutex<Vec<String>>>,
    }

    #[async_trait]
    impl Hook for TrackerHook {
        async fn on_session_start(&self) -> Result<(), anyhow::Error> {
            self.calls
                .lock()
                .unwrap()
                .push(format!("{}:session_start", self.name));
            Ok(())
        }

        async fn pre_turn(&self) -> Result<HookResult, anyhow::Error> {
            self.calls
                .lock()
                .unwrap()
                .push(format!("{}:pre_turn", self.name));
            if self.name == "deny" {
                Ok(HookResult {
                    allow: false,
                    message: "denied".to_string(),
                })
            } else {
                Ok(HookResult {
                    allow: true,
                    message: String::new(),
                })
            }
        }

        async fn pre_tool_call(&self, _tool_call: &ToolCall) -> Result<HookResult, anyhow::Error> {
            self.calls
                .lock()
                .unwrap()
                .push(format!("{}:pre_tool_call", self.name));
            if self.name == "deny" {
                Ok(HookResult {
                    allow: false,
                    message: "denied".to_string(),
                })
            } else {
                Ok(HookResult {
                    allow: true,
                    message: String::new(),
                })
            }
        }

        async fn post_tool_call(&self, _result: &ToolResult) -> Result<(), anyhow::Error> {
            self.calls
                .lock()
                .unwrap()
                .push(format!("{}:post_tool_call", self.name));
            Ok(())
        }

        async fn on_tool_error(
            &self,
            _error: &anyhow::Error,
        ) -> Result<(HookResult, Option<serde_json::Value>), anyhow::Error> {
            self.calls
                .lock()
                .unwrap()
                .push(format!("{}:on_tool_error", self.name));
            if self.name == "recover" {
                Ok((
                    HookResult {
                        allow: true,
                        message: "recovered".to_string(),
                    },
                    Some(serde_json::json!({"recovered": true})),
                ))
            } else {
                Ok((
                    HookResult {
                        allow: false,
                        message: "not recovered".to_string(),
                    },
                    None,
                ))
            }
        }

        async fn on_interaction(
            &self,
            _questions: &[AskQuestionEntry],
        ) -> Result<Option<QuestionHookResult>, anyhow::Error> {
            self.calls
                .lock()
                .unwrap()
                .push(format!("{}:on_interaction", self.name));
            if self.name == "answer" {
                Ok(Some(QuestionHookResult {
                    responses: vec![],
                    cancelled: false,
                }))
            } else {
                Ok(None)
            }
        }
    }

    #[tokio::test]
    async fn test_dispatch_session_start() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let runner = HookRunner::new();
        runner
            .register(Arc::new(TrackerHook {
                name: "h1".to_string(),
                calls: calls.clone(),
            }))
            .await;
        runner
            .register(Arc::new(TrackerHook {
                name: "h2".to_string(),
                calls: calls.clone(),
            }))
            .await;

        runner.dispatch_session_start().await.unwrap();

        let recorded = calls.lock().unwrap().clone();
        assert_eq!(recorded, vec!["h1:session_start", "h2:session_start"]);
    }

    #[tokio::test]
    async fn test_dispatch_pre_turn_allow() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let runner = HookRunner::new();
        runner
            .register(Arc::new(TrackerHook {
                name: "h1".to_string(),
                calls: calls.clone(),
            }))
            .await;
        runner
            .register(Arc::new(TrackerHook {
                name: "h2".to_string(),
                calls: calls.clone(),
            }))
            .await;

        let res = runner.dispatch_pre_turn().await.unwrap();
        assert!(res.allow);

        let recorded = calls.lock().unwrap().clone();
        assert_eq!(recorded, vec!["h1:pre_turn", "h2:pre_turn"]);
    }

    #[tokio::test]
    async fn test_dispatch_pre_turn_deny_short_circuits() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let runner = HookRunner::new();
        runner
            .register(Arc::new(TrackerHook {
                name: "h1".to_string(),
                calls: calls.clone(),
            }))
            .await;
        runner
            .register(Arc::new(TrackerHook {
                name: "deny".to_string(),
                calls: calls.clone(),
            }))
            .await;
        runner
            .register(Arc::new(TrackerHook {
                name: "h2".to_string(),
                calls: calls.clone(),
            }))
            .await;

        let res = runner.dispatch_pre_turn().await.unwrap();
        assert!(!res.allow);
        assert_eq!(res.message, "denied");

        let recorded = calls.lock().unwrap().clone();
        assert_eq!(recorded, vec!["h1:pre_turn", "deny:pre_turn"]);
    }

    #[tokio::test]
    async fn test_dispatch_pre_tool_call_deny_short_circuits() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let runner = HookRunner::new();
        runner
            .register(Arc::new(TrackerHook {
                name: "h1".to_string(),
                calls: calls.clone(),
            }))
            .await;
        runner
            .register(Arc::new(TrackerHook {
                name: "deny".to_string(),
                calls: calls.clone(),
            }))
            .await;
        runner
            .register(Arc::new(TrackerHook {
                name: "h2".to_string(),
                calls: calls.clone(),
            }))
            .await;

        let tool_call = ToolCall {
            id: "call_1".to_string(),
            name: "tool_1".to_string(),
            args: serde_json::Value::Null,
            canonical_path: None,
        };
        let res = runner.dispatch_pre_tool_call(&tool_call).await.unwrap();
        assert!(!res.allow);
        assert_eq!(res.message, "denied");

        let recorded = calls.lock().unwrap().clone();
        assert_eq!(recorded, vec!["h1:pre_tool_call", "deny:pre_tool_call"]);
    }

    #[tokio::test]
    async fn test_dispatch_post_tool_call() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let runner = HookRunner::new();
        runner
            .register(Arc::new(TrackerHook {
                name: "h1".to_string(),
                calls: calls.clone(),
            }))
            .await;

        let res = ToolResult {
            name: "tool_1".to_string(),
            id: Some("call_1".to_string()),
            result: Some(serde_json::Value::Null),
            error: None,
        };
        runner.dispatch_post_tool_call(&res).await.unwrap();

        let recorded = calls.lock().unwrap().clone();
        assert_eq!(recorded, vec!["h1:post_tool_call"]);
    }

    #[tokio::test]
    async fn test_dispatch_on_tool_error_recovery_short_circuits() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let runner = HookRunner::new();
        runner
            .register(Arc::new(TrackerHook {
                name: "h1".to_string(),
                calls: calls.clone(),
            }))
            .await;
        runner
            .register(Arc::new(TrackerHook {
                name: "recover".to_string(),
                calls: calls.clone(),
            }))
            .await;
        runner
            .register(Arc::new(TrackerHook {
                name: "h2".to_string(),
                calls: calls.clone(),
            }))
            .await;

        let err = anyhow::anyhow!("error occurred");
        let (res, val) = runner.dispatch_on_tool_error(&err).await.unwrap();
        assert!(res.allow);
        assert_eq!(res.message, "recovered");
        assert_eq!(val.unwrap(), serde_json::json!({"recovered": true}));

        let recorded = calls.lock().unwrap().clone();
        assert_eq!(recorded, vec!["h1:on_tool_error", "recover:on_tool_error"]);
    }

    #[tokio::test]
    async fn test_dispatch_interaction_short_circuits() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let runner = HookRunner::new();
        runner
            .register(Arc::new(TrackerHook {
                name: "h1".to_string(),
                calls: calls.clone(),
            }))
            .await;
        runner
            .register(Arc::new(TrackerHook {
                name: "answer".to_string(),
                calls: calls.clone(),
            }))
            .await;
        runner
            .register(Arc::new(TrackerHook {
                name: "h2".to_string(),
                calls: calls.clone(),
            }))
            .await;

        let entries = vec![];
        let res = runner.dispatch_interaction(&entries).await.unwrap();
        assert!(res.is_some());

        let recorded = calls.lock().unwrap().clone();
        assert_eq!(recorded, vec!["h1:on_interaction", "answer:on_interaction"]);
    }
}
