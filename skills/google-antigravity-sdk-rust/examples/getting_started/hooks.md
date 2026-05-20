# Lifecycle Hooks

Hooks in the Google Antigravity Rust SDK allow you to intercept, monitor, and override various lifecycle events of the agent session.

## Hook Trait Definition

To create custom hooks, implement the `Hook` trait:

```rust
use antigravity_sdk_rust::hooks::Hook;
use antigravity_sdk_rust::types::{AskQuestionEntry, HookResult, QuestionHookResult, ToolCall, ToolResult};
use async_trait::async_trait;

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
```

---

## Code Example

Below is an example showing how to declare and register a custom logging Hook inside `AgentConfig`:

```rust
use antigravity_sdk_rust::agent::{Agent, AgentConfig};
use antigravity_sdk_rust::hooks::Hook;
use antigravity_sdk_rust::policy;
use antigravity_sdk_rust::types::{HookResult, ToolCall, ToolResult};
use async_trait::async_trait;
use std::sync::Arc;

// 1. Define custom hook struct
struct LoggerHook;

#[async_trait]
impl Hook for LoggerHook {
    async fn on_session_start(&self) -> Result<(), anyhow::Error> {
        println!("[Hook] Session has successfully started!");
        Ok(())
    }

    async fn pre_turn(&self) -> Result<HookResult, anyhow::Error> {
        println!("[Hook] Preparing next turn...");
        Ok(HookResult {
            allow: true,
            message: String::new(),
        })
    }

    async fn pre_tool_call(&self, tool_call: &ToolCall) -> Result<HookResult, anyhow::Error> {
        println!("[Hook] Inspecting tool call request: {}", tool_call.name);
        Ok(HookResult {
            allow: true,
            message: String::new(),
        })
    }

    async fn post_tool_call(&self, result: &ToolResult) -> Result<(), anyhow::Error> {
        println!("[Hook] Tool call completed with result: {:?}", result.result);
        Ok(())
    }
}

// 2. Register hook inside configuration
#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let mut config = AgentConfig::default();
    config.policies = Some(vec![policy::allow_all()]);

    // Register our hook
    config.hooks.push(Arc::new(LoggerHook));

    let mut agent = Agent::new(config);
    agent.start().await?;

    let response = agent.chat("What files are in the current workspace?").await?;
    println!("Agent response: {}", response.text);

    agent.stop().await?;
    Ok(())
}
```
