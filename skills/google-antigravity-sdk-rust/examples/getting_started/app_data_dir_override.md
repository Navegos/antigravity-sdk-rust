# Application Data Directory Override

This example demonstrates how to override the default application data directory (`app_data_dir`) in `AgentConfig` to control where the agent stores generated artifacts (such as `task.md`), scratch files, and media on disk.

## Overriding Storage Directory

By default, the agent writes artifacts and scratchpad files to `~/.gemini/antigravity/brain/`. You can customize this storage location by providing an absolute path.

> [!IMPORTANT]
> **The path must be an absolute path.** Passing relative paths or unexpanded tildes (`~/`) will cause a validation error when starting the connection.

```rust
use antigravity_sdk_rust::agent::{Agent, AgentConfig};
use antigravity_sdk_rust::policy;
use antigravity_sdk_rust::types::GeminiConfig;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let mut config = AgentConfig::default();
    
    // Set custom absolute path for metadata and artifacts storage
    let custom_dir = "/tmp/custom_antigravity_data".to_string();
    std::fs::create_dir_all(&custom_dir)?;
    config.app_data_dir = Some(custom_dir);

    config.policies = Some(vec![policy::allow_all()]);

    let mut agent = Agent::new(config);
    agent.start().await?;

    // Generated artifacts and scratch files will be saved inside the custom directory
    let response = agent.chat("Create an artifact named 'notes.md' summarizing our core tasks.").await?;
    println!("Agent response: {}", response.text);

    agent.stop().await?;
    Ok(())
}
```
