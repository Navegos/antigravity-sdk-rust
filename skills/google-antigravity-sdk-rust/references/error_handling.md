# Error Handling & Lifecycle Hooks

This guide explains how errors are handled, propagated, and intercepted within the Google Antigravity Rust SDK.

## Error Propagation

API functions (e.g. `Agent::start()`, `Agent::chat()`, `Conversation::chat_to_completion()`) return standard Rust `Result<T, anyhow::Error>` types. 

Critical errors that stop execution:
* **Harness Handshake Failure**: Subprocess terminates before stdin/stdout exchange finishes, or Port information is invalid.
* **WebSocket Disconnection**: Network or websocket client errors during an active stream.
* **Platform Errors**: Server-side credentials invalid, rate limits exceeded (mapped from HTTP status codes inside `StepUpdate` streams).

---

## Intercepting Errors & Lifecycle Events: The Hook Trait

The SDK provides a thread-safe lifecycle hook mechanism. Registering structures implementing the `Hook` trait allows you to audit execution, print diagnostic traces, or handle tool failures.

### The Hook Trait Definition

```rust
use antigravity_sdk_rust::types::{ToolCall, ToolResult, HookResult, AskQuestionEntry, QuestionHookResult};
use async_trait::async_trait;

#[async_trait]
pub trait Hook: Send + Sync {
    /// Triggered when the subprocess is initialized and the session starts.
    async fn on_session_start(&self) -> Result<(), anyhow::Error> {
        Ok(())
    }

    /// Triggered before a model turn starts.
    async fn pre_turn(&self, _prompt: &str) -> Result<(), anyhow::Error> {
        Ok(())
    }

    /// Triggered before a tool execution. Returns a HookResult indicating whether to allow or block.
    async fn pre_tool_call(&self, _tool_call: &ToolCall) -> Result<HookResult, anyhow::Error> {
        Ok(HookResult { allow: true, message: String::new() })
    }

    /// Triggered after a tool execution successfully completes.
    async fn post_tool_call(&self, _result: &ToolResult) -> Result<(), anyhow::Error> {
        Ok(())
    }

    /// Triggered when a tool execution encounters an error.
    async fn on_tool_error(&self, _error: &anyhow::Error) -> Result<(), anyhow::Error> {
        Ok(())
    }

    /// Triggered when the agent prompts the user with interactive questions.
    async fn on_interaction(
        &self,
        _questions: &[AskQuestionEntry],
    ) -> Result<Option<QuestionHookResult>, anyhow::Error> {
        Ok(None)
    }
}
```

---

## Implementing a Diagnostic Logging Hook

Below is an implementation of a custom monitoring hook that logs tool execution results and captures error conditions:

```rust
use antigravity_sdk_rust::agent::{Agent, AgentConfig};
use antigravity_sdk_rust::hooks::Hook;
use antigravity_sdk_rust::types::{ToolCall, ToolResult, HookResult};
use async_trait::async_trait;
use std::sync::Arc;

struct DiagnosticLogger;

#[async_trait]
impl Hook for DiagnosticLogger {
    async fn pre_tool_call(&self, tool_call: &ToolCall) -> Result<HookResult, anyhow::Error> {
        println!("[HOOK] Executing Tool: {} with args: {}", tool_call.name, tool_call.args);
        Ok(HookResult {
            allow: true,
            message: String::new(),
        })
    }

    async fn post_tool_call(&self, result: &ToolResult) -> Result<(), anyhow::Error> {
        println!("[HOOK] Tool Complete: {} - Result: {:?}", result.name, result.result);
        Ok(())
    }

    async fn on_tool_error(&self, error: &anyhow::Error) -> Result<(), anyhow::Error> {
        eprintln!("[HOOK ERROR] Tool failed: {}", error);
        // Custom telemetry or recovery logic can go here
        Ok(())
    }
}

// Registering the hook in AgentConfig
let mut config = AgentConfig::default();
config.hooks.push(Arc::new(DiagnosticLogger));
```
