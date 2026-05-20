# Streaming Example Walkthrough

This walkthrough demonstrates how to stream chat responses, distinguishing reasoning thought blocks from response text and tool execution notifications in real time.

## Example Source Code

The full implementation is located in [examples/streaming.rs](file:///Volumes/goldcoders/antigravity-sdk-rust/examples/streaming.rs).

Core streaming loop:

```rust
use antigravity_sdk_rust::agent::{Agent, AgentConfig};
use antigravity_sdk_rust::policy;
use antigravity_sdk_rust::types::{GeminiConfig, StreamChunk};
use futures_util::StreamExt;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let mut config = AgentConfig::default();
    
    let mut gemini_config = GeminiConfig::default();
    gemini_config.models.default.name = "gemini-3.5-flash".to_string();
    config.gemini_config = gemini_config;
    config.policies = Some(vec![policy::allow_all()]);

    let mut agent = Agent::new(config);
    agent.start().await?;

    let prompt = "Explain quantum physics in one sentence. Show your thinking.";

    // 1. Get a mutable handle on the conversation
    let conversation = agent.conversation()?;
    
    // 2. Start the chat stream
    let mut stream = conversation.chat(prompt).await?;

    println!("Agent response (Streaming):");
    
    // 3. Process incoming stream chunks
    while let Some(chunk_res) = stream.next().await {
        match chunk_res? {
            StreamChunk::Thought { text, .. } => {
                // Internal reasoning process from model
                print!("[Thought: {}]", text);
            }
            StreamChunk::Text { text, .. } => {
                // Rendered output answer
                print!("{}", text);
            }
            StreamChunk::ToolCall(call) => {
                // Warning: tool execution is about to start
                println!("\n[Executing Tool: {}]", call.name);
            }
        }
        std::io::Write::flush(&mut std::io::stdout())?;
    }
    println!();

    agent.stop().await?;
    Ok(())
}
```

---

## Detailed Explanation

1. **`conversation()` Handle**: Fetches the current thread-safe `Conversation` context from the agent.
2. **`conversation.chat()`**: Submits the prompt to the backend and returns a stream of `Result<StreamChunk, anyhow::Error>` items.
3. **`StreamChunk` Enums**:
   * **`StreamChunk::Thought`**: Outputs the reasoning steps (emitted by reasoning models when processing the prompt). Useful for displaying UI progress spinners or developer diagnostics.
   * **`StreamChunk::Text`**: The conversational message intended for the user.
   * **`StreamChunk::ToolCall`**: Contains the tool configuration (name, id, arguments) before execution.

---

## How to Run

```bash
export GEMINI_API_KEY="your_api_key_here"
cargo run --example streaming
```
