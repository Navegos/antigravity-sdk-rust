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

fn block_rm_predicate(tool_call: &ToolCall) -> bool {
    tool_call
        .args
        .get("CommandLine")
        .and_then(Value::as_str)
        .is_some_and(|cmd| cmd.contains("rm"))
}

fn critical_file_predicate(tool_call: &ToolCall) -> bool {
    tool_call
        .args
        .get("TargetFile")
        .or_else(|| tool_call.args.get("target_file"))
        .or_else(|| tool_call.args.get("path"))
        .and_then(Value::as_str)
        .is_some_and(|p| {
            std::path::Path::new(p)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("key"))
                || p.contains("production")
        })
}

fn programmatic_approval_handler(tool_call: &ToolCall) -> bool {
    println!(
        "\n  [ASK_USER Handler] Intercepted request for tool: {}",
        tool_call.name
    );
    println!("  [ASK_USER Handler] Target arguments: {}", tool_call.args);
    println!("  [ASK_USER Handler] Simulating user review... Decision: DENY.");
    false
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let mut config = AgentConfig::default();
    
    let mut gemini_config = GeminiConfig::default();
    gemini_config.models.default.name = "gemini-3.5-flash".to_string();
    config.gemini_config = gemini_config;

    // Configure policies using the recommended "Deny by Default" posture.
    let policies = vec![
        // 1. Deny everything by default
        policy::deny_all(),
        // 2. Allow listing directories
        policy::allow("LIST_DIR"),
        // 3. Allow running commands, but block dangerous 'rm' commands
        Policy::new(
            "RUN_COMMAND".to_string(),
            Decision::Deny,
            Some(Arc::new(block_rm_predicate)),
            None,
            "block-rm".to_string(),
        ),
        // Fallback: Allow general RUN_COMMAND calls if they don't match the rm block predicate
        policy::allow("RUN_COMMAND"),
        // 4. Allow editing/creating files, but ask the user first if it's a critical file
        Policy::new(
            "CREATE_FILE".to_string(),
            Decision::AskUser,
            Some(Arc::new(critical_file_predicate)),
            Some(Arc::new(programmatic_approval_handler)),
            "ask-for-critical-writes".to_string(),
        ),
        policy::allow("CREATE_FILE"),
    ];
    config.policies = Some(policies);

    let mut agent = Agent::new(config);
    agent.start().await?;

    // Safe action - allowed
    agent.chat("List files in the directory.").await?;

    // Blocked action - rm blocked by policy
    agent.chat("Delete all files using rm -rf.").await?;

    // Blocked action - production.key write rejected by custom handler
    agent.chat("Create a new configuration file named production.key with content 'debug=true'.").await?;

    agent.stop().await?;
    Ok(())
}
```

---

## Detailed Policy Assessment

1. **Deny All**: A wildcard deny policy. In the bucket-based priority system, specific-tool rules (like `allow("LIST_DIR")`) always take priority over wildcard rules, so this acts as a fallback blocking any tools not explicitly allowed.
2. **`LIST_DIR`**: Placed next, whitelisting directory viewing.
3. **`RUN_COMMAND` with Predicate**: 
   * Pre-filters tool arguments.
   * If `CommandLine` includes `"rm"`, evaluation returns `Decision::Deny`, blocking execution instantly.
   * If the predicate returns `false` (no `rm` found), this deny policy is skipped. The specific allow policy for `RUN_COMMAND` (bucket 2) then matches and permits execution.
4. **`CREATE_FILE` with Interceptor**:
   * Evaluates if the path ends in `.key` or contains `production`.
   * If it matches, decision is `Decision::AskUser`, redirecting execution flow to `programmatic_approval_handler`.
   * The handler returns `false` (simulating user rejection), blocking the tool call and returning a prompt refusal.

---

## How to Run

```bash
export GEMINI_API_KEY="your_api_key_here"
cargo run --example policies
```
