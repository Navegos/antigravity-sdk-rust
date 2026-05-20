use antigravity_sdk_rust::agent::{Agent, AgentConfig};
use antigravity_sdk_rust::policy;
use antigravity_sdk_rust::types::GeminiConfig;
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

    let mut config1 = AgentConfig::default();
    if let Ok(harness_path) = std::env::var("ANTIGRAVITY_HARNESS_PATH") {
        config1.binary_path = Some(harness_path);
    }
    let mut gemini_config1 = GeminiConfig::default();
    if let Ok(api_key) = std::env::var("GEMINI_API_KEY") {
        gemini_config1.api_key = Some(api_key);
    }
    gemini_config1.models.default.name = "gemini-3.5-flash".to_string();
    config1.gemini_config = gemini_config1;
    config1.policies = Some(vec![policy::allow_all()]);
    config1.save_dir = Some(save_dir_str.clone());

    let conversation_id = {
        let mut agent1 = Agent::new(config1);
        agent1.start().await?;

        let prompt1 = "Remember this: my favorite color is blue.";
        println!("  User: {}", prompt1);
        let response1 = agent1.chat(prompt1).await?;
        println!("  Agent: {}", response1.text);

        let conv_id = agent1
            .conversation_id()
            .ok_or_else(|| anyhow::anyhow!("Failed to retrieve conversation ID"))?;
        println!("  Assigned conversation ID: {}", conv_id);

        agent1.stop().await?;
        println!("  Session 1 ended.\n");
        conv_id
    };

    println!("  === Session 2: resuming and verifying recall ===");

    let mut config2 = AgentConfig::default();
    if let Ok(harness_path) = std::env::var("ANTIGRAVITY_HARNESS_PATH") {
        config2.binary_path = Some(harness_path);
    }
    let mut gemini_config2 = GeminiConfig::default();
    if let Ok(api_key) = std::env::var("GEMINI_API_KEY") {
        gemini_config2.api_key = Some(api_key);
    }
    gemini_config2.models.default.name = "gemini-3.5-flash".to_string();
    config2.gemini_config = gemini_config2;
    config2.policies = Some(vec![policy::allow_all()]);
    config2.save_dir = Some(save_dir_str.clone());
    config2.conversation_id = Some(conversation_id);

    let mut agent2 = Agent::new(config2);
    agent2.start().await?;

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
