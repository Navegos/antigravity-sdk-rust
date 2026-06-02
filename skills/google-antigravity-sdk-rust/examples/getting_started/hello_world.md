# Hello World Example

This example demonstrates how to perform a basic chat interaction, stream text and internal thoughts, and build an interactive terminal chat loop using the Google Antigravity Rust SDK.

## Basic Chat

```rust
use antigravity_sdk_rust::agent::{Agent, AgentConfig};
use antigravity_sdk_rust::policy;
use antigravity_sdk_rust::types::GeminiConfig;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let mut config = AgentConfig::default();
    
    // Configure Gemini model
    let mut gemini_config = GeminiConfig::default();
    gemini_config.models.default.name = "gemini-3.5-flash".to_string();
    config.gemini_config = gemini_config;
    config.policies = Some(vec![policy::allow_all()]);

    let mut agent = Agent::new(config);
    agent.start().await?;

    let response = agent.chat("Hello, World!").await?;
    println!("Agent response: {}", response.text);

    agent.stop().await?;
    Ok(())
}
```

---

## Streaming Response and Thoughts

You can stream tokens and reasoning thoughts as they arrive by accessing the conversation handler:

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

    let conversation = agent.conversation();
    let mut stream = conversation.chat("Solve this math riddle: I am an odd number. Take away one letter and I become even. What number am I?").await?;

    while let Some(chunk_res) = stream.next().await {
        match chunk_res? {
            StreamChunk::Thought { text, .. } => {
                // Internal reasoning process/thought
                print!("[Thought: {}]", text);
            }
            StreamChunk::Text { text, .. } => {
                // Final textual answer
                print!("{}", text);
            }
            StreamChunk::ToolCall(call) => {
                println!("\n[Tool call requested: {}]", call.name);
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

## Interactive Chat Loop

The Rust SDK includes a prebuilt `run_interactive_loop()` helper in the `interactive` module to quickly run a console session.

Here is how you can run the prebuilt interactive loop:

```rust
use antigravity_sdk_rust::agent::{Agent, AgentConfig};
use antigravity_sdk_rust::interactive::run_interactive_loop;
use antigravity_sdk_rust::policy;
use antigravity_sdk_rust::types::GeminiConfig;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let mut config = AgentConfig::default();
    
    let mut gemini_config = GeminiConfig::default();
    gemini_config.models.default.name = "gemini-3.5-flash".to_string();
    config.gemini_config = gemini_config;
    config.policies = Some(vec![policy::allow_all()]);

    let mut agent = Agent::new(config);
    agent.start().await?;

    // Runs a full interactive REPL loop in the terminal
    run_interactive_loop(&agent).await?;

    agent.stop().await?;
    Ok(())
}
```

### Custom Console Loop (Alternative)

If you need full control over the inputs, prompt format, or error handling, you can implement your own custom console loop using standard prompt readings:

```rust
use antigravity_sdk_rust::agent::{Agent, AgentConfig};
use antigravity_sdk_rust::policy;
use antigravity_sdk_rust::types::GeminiConfig;
use std::io::{self, Write};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let mut config = AgentConfig::default();
    
    let mut gemini_config = GeminiConfig::default();
    gemini_config.models.default.name = "gemini-3.5-flash".to_string();
    config.gemini_config = gemini_config;
    config.policies = Some(vec![policy::allow_all()]);

    let mut agent = Agent::new(config);
    agent.start().await?;

    println!("Starting interactive loop. Type 'exit' or 'quit' to end.");

    let stdin = io::stdin();
    loop {
        print!("\nUser: ");
        io::stdout().flush()?;

        let mut input = String::new();
        stdin.read_line(&mut input)?;
        let trimmed = input.trim();

        if trimmed.eq_ignore_ascii_case("exit") || trimmed.eq_ignore_ascii_case("quit") {
            break;
        }

        if trimmed.is_empty() {
            continue;
        }

        let response = agent.chat(trimmed).await?;
        println!("Agent: {}", response.text);
    }

    agent.stop().await?;
    Ok(())
}
```
