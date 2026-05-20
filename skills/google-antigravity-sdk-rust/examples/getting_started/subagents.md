# Subagent Orchestration Example

This example walkthrough demonstrates how to authorize an agent to delegate tasks to subagents and monitor the subagent execution lifecycle using hooks.

## Code Example

The complete implementation can be found in [examples/subagents.rs](file:///Volumes/goldcoders/antigravity-sdk-rust/examples/subagents.rs):

```rust
use antigravity_sdk_rust::agent::{Agent, AgentConfig};
use antigravity_sdk_rust::hooks::Hook;
use antigravity_sdk_rust::policy;
use antigravity_sdk_rust::types::{
    BuiltinTools, CapabilitiesConfig, GeminiConfig, HookResult, ToolCall, ToolResult,
};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

// Hook structure to log subagent activation
struct SubagentHook {
    subagent_active: Arc<AtomicBool>,
}

impl Hook for SubagentHook {
    fn pre_tool_call<'a>(&'a self, tool_call: &'a ToolCall) -> impl std::future::Future<Output = Result<HookResult, anyhow::Error>> + Send {
        async move {
            if tool_call.name == "START_SUBAGENT" {
                self.subagent_active.store(true, Ordering::SeqCst);
                println!("\n--- [Hook] Spawning Subagent ---");
                println!("Arguments: {}\n", tool_call.args);
            } else {
                let indent = if self.subagent_active.load(Ordering::SeqCst) { "    " } else { "  " };
                println!("{}- [Start]: {} (ID: {})", indent, tool_call.name, tool_call.id);
            }
            Ok(HookResult { allow: true, message: String::new() })
        }
    }

    fn post_tool_call<'a>(&'a self, result: &'a ToolResult) -> impl std::future::Future<Output = Result<(), anyhow::Error>> + Send {
        async move {
            if result.name == "START_SUBAGENT" {
                self.subagent_active.store(false, Ordering::SeqCst);
                println!("\n--- [Hook] Subagent Finished ---\n");
            } else {
                let indent = if self.subagent_active.load(Ordering::SeqCst) { "    " } else { "  " };
                println!("{}- [Done]: {} ✅", indent, result.name);
            }
            Ok(())
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let mut config = AgentConfig::default();
    
    let mut gemini_config = GeminiConfig::default();
    gemini_config.models.default.name = "gemini-3.5-flash".to_string();
    config.gemini_config = gemini_config;

    // 1. Enable StartSubagent capability
    config.capabilities = CapabilitiesConfig {
        enabled_tools: Some(vec![
            BuiltinTools::StartSubagent,
            BuiltinTools::ListDir,
            BuiltinTools::ViewFile,
            BuiltinTools::Finish,
        ]),
        ..Default::default()
    };

    // 2. Register lifecycle monitoring Hook
    let subagent_active = Arc::new(AtomicBool::new(false));
    config.hooks.push(Arc::new(SubagentHook { subagent_active }));

    config.policies = Some(vec![policy::allow_all()]);

    let mut agent = Agent::new(config);
    agent.start().await?;

    let prompt = "Spawn a subagent to list the files in the current directory and tell me what you find.";
    let response = agent.chat(prompt).await?;
    println!("Agent Response: {}", response.text);

    agent.stop().await?;
    Ok(())
}
```

---

## Detailed Explanation

1. **Subagent Tool Capability**:
   * To allow an agent to trigger subagents, you must explicitly pass `BuiltinTools::StartSubagent` inside the `enabled_tools` array of `CapabilitiesConfig`.
   * When the agent decides to delegate, it triggers the built-in `START_SUBAGENT` tool.
2. **Lifecycle Monitoring**:
   * The hook checks if the intercepted tool call name is `START_SUBAGENT`.
   * If a subagent is active, it sets an atomic flag so that nested tool calls (invoked by the subagent process) are indented when printed to the console.
   * This provides trace visibility into multi-agent hierarchy executions.

---

## How to Run

```bash
export GEMINI_API_KEY="your_api_key_here"
cargo run --example subagents
```
