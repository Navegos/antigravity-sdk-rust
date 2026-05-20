use antigravity_sdk_rust::agent::Agent;
use std::time::SystemTime;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Initialize tracing subscriber
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    // Load environment variables from .env file if present
    dotenvy::dotenv().ok();

    // Create a temporary path for save_dir
    let mut save_dir = std::env::temp_dir();
    let epoch = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    save_dir.push(format!("agent_session_{}", epoch));
    let save_dir_str = save_dir.to_string_lossy().to_string();
    println!("  Save directory: {}", save_dir_str);

    // Ensure the save_dir exists
    std::fs::create_dir_all(&save_dir)?;

    println!("\n  === Session 1: establishing context ===");

    let harness_path = std::env::var("ANTIGRAVITY_HARNESS_PATH").ok();
    let api_key = std::env::var("GEMINI_API_KEY").ok();

    let mut builder1 = Agent::builder();
    if let Some(path) = harness_path.clone() {
        builder1 = builder1.binary_path(path);
    }
    if let Some(key) = api_key.clone() {
        builder1 = builder1.api_key(key);
    }

    let agent1 = builder1
        .default_model("gemini-3.5-flash")
        .allow_all()
        .save_dir(save_dir_str.clone())
        .build();

    let conversation_id = {
        let agent1 = agent1.start().await?;

        let prompt1 = "Remember this: my favorite color is blue.";
        println!("  User: {}", prompt1);
        let response1 = agent1.chat(prompt1).await?;
        println!("  Agent: {}", response1.text);

        let conv_id = agent1.conversation_id();
        println!("  Assigned conversation ID: {}", conv_id);

        agent1.stop().await?;
        println!("  Session 1 ended.\n");
        conv_id
    };

    println!("  === Session 2: resuming and verifying recall ===");

    let mut builder2 = Agent::builder();
    if let Some(path) = harness_path {
        builder2 = builder2.binary_path(path);
    }
    if let Some(key) = api_key {
        builder2 = builder2.api_key(key);
    }

    let agent2 = builder2
        .default_model("gemini-3.5-flash")
        .allow_all()
        .save_dir(save_dir_str.clone())
        .conversation_id(conversation_id)
        .build();

    let agent2 = agent2.start().await?;

    let prompt2 = "What is my favorite color?";
    println!("  User: {}", prompt2);
    let response2 = agent2.chat(prompt2).await?;
    println!("  Agent: {}", response2.text);

    agent2.stop().await?;
    println!("  Session 2 ended.");

    // Clean up temporary session directory
    let _ = std::fs::remove_dir_all(&save_dir);

    Ok(())
}
