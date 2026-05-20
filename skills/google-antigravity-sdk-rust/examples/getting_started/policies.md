# Safety Policies Example

This walkthrough illustrates how to build a robust policy layer to filter CLI commands and prompt before writing sensitive files.

## Code Example

The full implementation is located in [examples/policies.rs](file:///Volumes/goldcoders/antigravity-sdk-rust/examples/policies.rs):

```rust
use antigravity_sdk_rust::agent::{Agent, AgentConfig};
use antigravity_sdk_rust::policy::{self, Decision, Policy};
use antigravity_sdk_rust::types::{GeminiConfig, ToolCall};
use serde_json::Value;
use std::sync::Arc;

// Predicate to intercept dangerous shell commands containing 'rm'
fn block_rm_predicate(tool_call: &ToolCall) -> bool {
    tool_call.args.get("CommandLine")
        .and_then(Value::as_str)
        .is_some_and(|cmd| cmd.contains("rm"))
}

// Predicate to identify key files or production environments
fn critical_file_predicate(tool_call: &ToolCall) -> bool {
    tool_call.args.get("TargetFile")
        .and_then(Value::as_str)
        .is_some_and(|p| p.ends_with(".key") || p.contains("production"))
}

// Handler simulation for 'AskUser' decision
fn programmatic_approval_handler(tool_call: &ToolCall) -> bool {
    println!("[Policy Interceptor] Prompting for critical write: {}", tool_call.name);
    // Simulating user rejection
    false 
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let mut config = AgentConfig::default();
    
    let mut gemini_config = GeminiConfig::default();
    gemini_config.models.default.name = "gemini-3.5-flash".to_string();
    config.gemini_config = gemini_config;

    // Define policy list (evaluated in reverse order)
    let policies = vec![
        // 1. Deny everything by default
        policy::deny_all(),
        // 2. Allow directory listings
        policy::allow("LIST_DIR"),
        // 3. Deny commands containing 'rm', fallback to allow general commands
        Policy::new(
            "RUN_COMMAND".to_string(),
            Decision::Deny,
            Some(Arc::new(block_rm_predicate)),
            None,
            "block-rm".to_string(),
        ),
        policy::allow("RUN_COMMAND"),
        // 4. Prompt user before writes matching critical predicates, allow general writes
        Policy::new(
            "WRITE_TO_FILE".to_string(),
            Decision::AskUser,
            Some(Arc::new(critical_file_predicate)),
            Some(Arc::new(programmatic_approval_handler)),
            "ask-critical-writes".to_string(),
        ),
        policy::allow("WRITE_TO_FILE"),
    ];
    config.policies = Some(policies);

    let mut agent = Agent::new(config);
    agent.start().await?;

    // Safe action - allowed
    agent.chat("List files in the directory.").await?;

    // Blocked action - rm blocked by policy
    agent.chat("Delete all files using rm -rf.").await?;

    // Blocked action - production.key write rejected by custom handler
    agent.chat("Create a new file named production.key with secret data.").await?;

    agent.stop().await?;
    Ok(())
}
```

---

## Detailed Policy Assessment

1. **Deny All**: Placed first in the vector. Since evaluation order is reversed (bottom-to-top), it acts as a final safety fallback blocking unmapped tools.
2. **`LIST_DIR`**: Placed next, whitelisting directory viewing.
3. **`RUN_COMMAND` with Predicate**: 
   * Pre-filters tool arguments.
   * If `CommandLine` includes `"rm"`, evaluation returns `Decision::Deny`, blocking execution instantly.
   * If it passes, evaluation continues to the next policy (`policy::allow("RUN_COMMAND")`), which allows the tool to run.
4. **`WRITE_TO_FILE` with Interceptor**:
   * Evaluates if the path ends in `.key` or contains `production`.
   * If it matches, decision is `Decision::AskUser`, redirecting execution flow to `programmatic_approval_handler`.
   * The handler returns `false` (simulating user rejection), blocking the tool call and returning a prompt refusal.

---

## How to Run

```bash
export GEMINI_API_KEY="your_api_key_here"
cargo run --example policies
```
