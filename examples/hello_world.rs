use antigravity_sdk_rust::agent::{Agent, AgentConfig};
use antigravity_sdk_rust::policy;
use antigravity_sdk_rust::types::GeminiConfig;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Initialize tracing subscriber to print info/debug logs by default
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    // Load environment variables from .env file if present
    dotenvy::dotenv().ok();

    let mut config = AgentConfig::default();

    // Check if the user specified a binary path or check the environment variable
    if let Ok(harness_path) = std::env::var("ANTIGRAVITY_HARNESS_PATH") {
        config.binary_path = Some(harness_path);
    }

    // Configure Gemini Configuration with API Key and explicit model name
    let mut gemini_config = GeminiConfig::default();
    if let Ok(api_key) = std::env::var("GEMINI_API_KEY") {
        gemini_config.api_key = Some(api_key);
    }
    // Explicitly set the model to gemini-3.5-flash
    gemini_config.models.default.name = "gemini-3.5-flash".to_string();
    config.gemini_config = gemini_config;

    config.policies = Some(vec![policy::allow_all()]);

    let mut agent = Agent::new(config);
    println!("Starting agent...");
    agent.start().await?;

    let prompt = "Say 'Hello World!'";
    println!("  User: {}", prompt);

    let response = agent.chat(prompt).await?;
    println!("  Agent: {}", response.text);

    agent.stop().await?;
    Ok(())
}
