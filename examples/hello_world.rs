use antigravity_sdk_rust::agent::Agent;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Initialize tracing subscriber to print info/debug logs by default
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    // Load environment variables from .env file if present
    dotenvy::dotenv().ok();

    // Check if the user specified a binary path or check the environment variable
    let harness_path = std::env::var("ANTIGRAVITY_HARNESS_PATH").ok();
    let api_key = std::env::var("GEMINI_API_KEY").ok();

    let mut builder = Agent::builder();
    if let Some(path) = harness_path {
        builder = builder.binary_path(path);
    }
    if let Some(key) = api_key {
        builder = builder.api_key(key);
    }

    let agent = builder
        .default_model("gemini-3.5-flash")
        .allow_all()
        .build();

    println!("Starting agent...");
    let agent = agent.start().await?;

    let prompt = "Say 'Hello World!'";
    println!("  User: {}", prompt);

    let response = agent.chat(prompt).await?;
    println!("  Agent: {}", response.text);

    agent.stop().await?;
    Ok(())
}
