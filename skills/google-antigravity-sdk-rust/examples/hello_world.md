# Hello World Example Walkthrough

This walkthrough demonstrates how to build and run a basic single-turn conversational agent using the Google Antigravity Rust SDK.

## Example Source Code

The full implementation is located in [examples/hello_world.rs](file:///Volumes/goldcoders/antigravity-sdk-rust/examples/hello_world.rs):

```rust
use antigravity_sdk_rust::agent::{Agent, AgentConfig};
use antigravity_sdk_rust::policy;
use antigravity_sdk_rust::types::GeminiConfig;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // 1. Initialize logs
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    // Load .env variables
    dotenvy::dotenv().ok();

    // 2. Setup AgentConfig
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

    // Allow all tools (warning: not recommended for production)
    config.policies = Some(vec![policy::allow_all()]);

    // 3. Instantiate and Start Agent
    let mut agent = Agent::new(config);
    println!("Starting agent...");
    agent.start().await?;

    // 4. Send Message Turn
    let prompt = "Say 'Hello World!'";
    println!("  User: {}", prompt);

    let response = agent.chat(prompt).await?;
    println!("  Agent: {}", response.text);

    // 5. Cleanup Connection
    agent.stop().await?;
    Ok(())
}
```

---

## Detailed Explanation

1. **Logging Initialization**: Configures `tracing-subscriber` to print agent status updates (such as subprocess connection details) directly to stdout.
2. **Environment Variables**: Checks for `ANTIGRAVITY_HARNESS_PATH` (to resolve `localharness`) and `GEMINI_API_KEY`.
3. **Agent Configuration**:
   * Sets model to `gemini-3.5-flash`.
   * Adds `policy::allow_all()` to bypass tool confirmation blocks for simple queries.
4. **Agent Lifecycle**:
   * `agent.start()` spawns the local subprocess helper, completes the length-prefixed protocol handshake over stdin/stdout, and starts the WebSocket server connection.
   * `agent.chat()` sends the prompt and blocks asynchronously until the model outputs a final response.
   * `agent.stop()` cleanly terminates the WebSocket connection and kills the helper subprocess.

---

## How to Run

1. Supply your Gemini API key in the environment:
   ```bash
   export GEMINI_API_KEY="your_api_key_here"
   ```
2. Execute the cargo run command:
   ```bash
   cargo run --example hello_world
   ```
