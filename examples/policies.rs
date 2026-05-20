use antigravity_sdk_rust::agent::{Agent, AgentConfig};
use antigravity_sdk_rust::policy::{self, Decision, Policy};
use antigravity_sdk_rust::types::{GeminiConfig, ToolCall};
use serde_json::Value;
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

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
    // Initialize tracing subscriber
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    // Load environment variables from .env file if present
    dotenvy::dotenv().ok();

    let mut config = AgentConfig::default();

    if let Ok(harness_path) = std::env::var("ANTIGRAVITY_HARNESS_PATH") {
        config.binary_path = Some(harness_path);
    }

    let mut gemini_config = GeminiConfig::default();
    if let Ok(api_key) = std::env::var("GEMINI_API_KEY") {
        gemini_config.api_key = Some(api_key);
    }
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
            "WRITE_TO_FILE".to_string(),
            Decision::AskUser,
            Some(Arc::new(critical_file_predicate)),
            Some(Arc::new(programmatic_approval_handler)),
            "ask-for-critical-writes".to_string(),
        ),
        policy::allow("WRITE_TO_FILE"),
    ];
    config.policies = Some(policies);

    let mut agent = Agent::new(config);
    println!("Starting agent...");
    agent.start().await?;

    println!("\n  Chatting with agent...");

    // 1. Try a safe command (should be allowed)
    let prompt1 = "List the files in the current directory.";
    println!("\n  User: {}", prompt1);
    let response1 = agent.chat(prompt1).await?;
    println!("  Agent: {}", response1.text);

    // 2. Try a dangerous command (should be denied by policy)
    let prompt2 = "Delete all files using rm -rf.";
    println!("\n  User: {}", prompt2);
    let response2 = agent.chat(prompt2).await?;
    println!("  Agent: {}", response2.text);

    // 3. Try creating a critical file (triggers programmatic ask_user handler)
    let prompt3 = "Create a new configuration file named production.key with content 'debug=true'.";
    println!("\n  User: {}", prompt3);
    let response3 = agent.chat(prompt3).await?;
    println!("  Agent: {}", response3.text);

    agent.stop().await?;
    Ok(())
}
