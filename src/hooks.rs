use crate::types::{AskQuestionEntry, HookResult, QuestionHookResult, ToolCall, ToolResult};
use async_trait::async_trait;
use std::sync::Arc;

#[async_trait]
pub trait Hook: Send + Sync {
    async fn on_session_start(&self) -> Result<(), anyhow::Error> {
        Ok(())
    }
    async fn pre_turn(&self) -> Result<HookResult, anyhow::Error> {
        Ok(HookResult {
            allow: true,
            message: String::new(),
        })
    }
    async fn pre_tool_call(&self, _tool_call: &ToolCall) -> Result<HookResult, anyhow::Error> {
        Ok(HookResult {
            allow: true,
            message: String::new(),
        })
    }
    async fn post_tool_call(&self, _result: &ToolResult) -> Result<(), anyhow::Error> {
        Ok(())
    }
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
    async fn on_interaction(
        &self,
        _questions: &[AskQuestionEntry],
    ) -> Result<Option<QuestionHookResult>, anyhow::Error> {
        Ok(None)
    }
}

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
