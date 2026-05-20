use antigravity_sdk_rust::agent::{Agent, AgentConfig};
use antigravity_sdk_rust::policy;
use antigravity_sdk_rust::types::{GeminiConfig, StreamChunk};
use futures_util::StreamExt;
use tracing_subscriber::EnvFilter;

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

    config.policies = Some(vec![policy::allow_all()]);

    let mut agent = Agent::new(config);
    println!("Starting agent...");
    agent.start().await?;

    let prompt = "Solve this riddle: I speak without a mouth and hear without ears. I have no body, but I come alive with wind. What am I? Explain your reasoning.";
    println!("\n  User: {}\n", prompt);

    let conversation = agent.conversation()?;
    let mut stream = conversation.chat(prompt).await?;

    println!("  Agent (Streaming response):");
    println!("  -------------------------------------------------------");

    while let Some(chunk_res) = stream.next().await {
        match chunk_res? {
            StreamChunk::Thought { text, .. } => {
                // Print thought chunks (e.g. in grey or wrapped in brackets if desired, or directly)
                print!("{}", text);
                std::io::Write::flush(&mut std::io::stdout())?;
            }
            StreamChunk::Text { text, .. } => {
                // Print response text chunks
                print!("{}", text);
                std::io::Write::flush(&mut std::io::stdout())?;
            }
            StreamChunk::ToolCall(call) => {
                println!(
                    "\n[Tool Call Requested: {} with args: {}]",
                    call.name, call.args
                );
            }
        }
    }
    println!("\n  -------------------------------------------------------\n");

    agent.stop().await?;
    Ok(())
}
