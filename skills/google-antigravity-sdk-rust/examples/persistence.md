# Session Persistence Example Walkthrough

This walkthrough demonstrates how to cache conversation states locally and resume a session across different agent lifecycles.

## Example Source Code

The full implementation is located in [examples/persistence.rs](file:///Volumes/goldcoders/antigravity-sdk-rust/examples/persistence.rs). 

Key steps to achieve persistence:

```rust
use antigravity_sdk_rust::agent::{Agent, AgentConfig};
use antigravity_sdk_rust::policy;
use antigravity_sdk_rust::types::GeminiConfig;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let save_dir = "/tmp/agent_session_123".to_string();
    std::fs::create_dir_all(&save_dir)?;

    // ============================================
    // Session 1: Establish Context
    // ============================================
    let mut config1 = AgentConfig::default();
    config1.save_dir = Some(save_dir.clone());
    config1.policies = Some(vec![policy::allow_all()]);
    
    let mut gemini_config1 = GeminiConfig::default();
    gemini_config1.models.default.name = "gemini-3.5-flash".to_string();
    config1.gemini_config = gemini_config1;

    let conversation_id = {
        let mut agent1 = Agent::new(config1);
        agent1.start().await?;

        agent1.chat("Remember this: my favorite color is blue.").await?;

        // Retrieve the generated conversation ID
        let conv_id = agent1.conversation_id().unwrap();
        agent1.stop().await?;
        conv_id
    };

    // ============================================
    // Session 2: Resume & Recall Context
    // ============================================
    let mut config2 = AgentConfig::default();
    config2.save_dir = Some(save_dir.clone());
    config2.conversation_id = Some(conversation_id);
    config2.policies = Some(vec![policy::allow_all()]);

    let mut gemini_config2 = GeminiConfig::default();
    gemini_config2.models.default.name = "gemini-3.5-flash".to_string();
    config2.gemini_config = gemini_config2;

    let mut agent2 = Agent::new(config2);
    agent2.start().await?;

    let response = agent2.chat("What is my favorite color?").await?;
    println!("Resumed Agent Response: {}", response.text); // Should remember "blue"

    agent2.stop().await?;
    Ok(())
}
```

---

## Detailed Explanation

1. **`save_dir` Configuration**: Specifies a path where session history, logs, and artifacts are serialized.
2. **First Session**:
   * Starts a new conversation.
   * `agent.conversation_id()` retrieves the session identifier.
   * We shut down the agent cleanly using `agent.stop()`.
3. **Resuming Session**:
   * We instantiate a second agent, supplying the same `save_dir`.
   * We pass the previous conversation identifier using `config.conversation_id = Some(conversation_id)`.
   * Upon calling `agent.start()`, the underlying harness loads the past conversation history.
   * The agent retains full recollection of context from Turn 1.

---

## How to Run

```bash
export GEMINI_API_KEY="your_api_key_here"
cargo run --example persistence
```
