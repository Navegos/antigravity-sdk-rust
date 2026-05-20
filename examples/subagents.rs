use antigravity_sdk_rust::agent::{Agent, AgentConfig};
use antigravity_sdk_rust::hooks::Hook;
use antigravity_sdk_rust::policy;
use antigravity_sdk_rust::types::{
    BuiltinTools, CapabilitiesConfig, GeminiConfig, HookResult, ToolCall, ToolResult,
};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tracing_subscriber::EnvFilter;

struct SubagentHook {
    subagent_active: Arc<AtomicBool>,
}

impl Hook for SubagentHook {
    async fn pre_tool_call(&self, tool_call: &ToolCall) -> Result<HookResult, anyhow::Error> {
        if tool_call.name == "START_SUBAGENT" {
            self.subagent_active.store(true, Ordering::SeqCst);
            println!("\n  --- 🤖 [Hook] Spawning Subagent ---");
            println!("  Arguments: {}\n", tool_call.args);
        } else {
            let indent = if self.subagent_active.load(Ordering::SeqCst) {
                "    "
            } else {
                "  "
            };
            println!(
                "{}- [Start]: {} (ID: {})",
                indent, tool_call.name, tool_call.id
            );
        }
        Ok(HookResult {
            allow: true,
            message: String::new(),
        })
    }

    async fn post_tool_call(&self, result: &ToolResult) -> Result<(), anyhow::Error> {
        if result.name == "START_SUBAGENT" {
            self.subagent_active.store(false, Ordering::SeqCst);
            println!("\n  --- 🤖 [Hook] Subagent Finished ---");
            println!("  Result: {:?}\n", result.result);
        } else {
            let indent = if self.subagent_active.load(Ordering::SeqCst) {
                "    "
            } else {
                "  "
            };
            println!(
                "{}- [Done]: {} (ID: {}) ✅",
                indent,
                result.name,
                result.id.as_deref().unwrap_or("")
            );
        }
        Ok(())
    }
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

    // Enable subagents capability and file viewing
    let capabilities = CapabilitiesConfig {
        enabled_tools: Some(vec![
            BuiltinTools::StartSubagent,
            BuiltinTools::ListDir,
            BuiltinTools::ViewFile,
            BuiltinTools::Finish,
        ]),
        ..Default::default()
    };
    config.capabilities = capabilities;

    // Add Hook for visibility
    let subagent_active = Arc::new(AtomicBool::new(false));
    config
        .hooks
        .push(Arc::new(SubagentHook { subagent_active }));

    // Allow tools
    config.policies = Some(vec![policy::allow_all()]);

    let mut agent = Agent::new(config);
    println!("Starting agent...");
    agent.start().await?;

    let prompt = "Use a subagent to research the files in the current directory. \
                  Delegate the task of listing the directory to the subagent, and then \
                  tell me what files you found.";

    println!("  User: {}", prompt);
    let response = agent.chat(prompt).await?;
    println!("\n  Agent:\n{}", response.text);

    agent.stop().await?;
    Ok(())
}
