# Google Antigravity Rust SDK

The Google Antigravity SDK is a Rust library for building AI agents powered by Antigravity and Gemini. It provides a secure, scalable, and stateful infrastructure layer that abstracts the agentic loop, letting you focus on what your agent *does* rather than how it runs.

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
antigravity-sdk-rust = { path = "path/to/antigravity-sdk-rust" }
tokio = { version = "1", features = ["full"] }
```

## Quickstart

Get started by running the hello_world example:

```sh
export GEMINI_API_KEY="your_api_key_here"
cargo run --example hello_world
```

## Concepts

### Simple Agent

The `Agent` struct manages the full lifecycle — binary discovery, tool wiring, hook registration, and policy defaults.

```rust
use antigravity_sdk_rust::agent::{Agent, AgentConfig};
use antigravity_sdk_rust::policy;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let mut config = AgentConfig::default();
    config.policies = Some(vec![policy::allow_all()]);

    let mut agent = Agent::new(config);
    agent.start().await?;

    let response = agent.chat("Say 'Hello World!'").await?;
    println!("Agent: {}", response.text);

    agent.stop().await?;
    Ok(())
}
```

### Advanced Usage with Conversation

For full control over the connection lifecycle, use `Conversation` with a `ConnectionStrategy` directly. `Conversation` is a stateful session that accumulates step history:

```rust
use antigravity_sdk_rust::agent::AgentConfig;
use antigravity_sdk_rust::conversation::Conversation;
use antigravity_sdk_rust::local::LocalConnectionStrategy;
use antigravity_sdk_rust::tools::ToolRunner;
use antigravity_sdk_rust::types::GeminiConfig;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let tool_runner = ToolRunner::new();
    let strategy = LocalConnectionStrategy::new(
        "localharness".to_string(), // path to harness
        GeminiConfig::default(),
        Default::default(),
        None,
        None,
        vec![],
        vec![],
        Some(tool_runner),
        None,
        "my_conversation_id".to_string(),
    );

    let connection = strategy.connect().await?;
    let conversation = Conversation::new(Arc::new(connection), None);

    let response = conversation.chat_to_completion("What files are here?").await?;
    println!("Agent: {}", response.text);

    Ok(())
}
```

## Features

### Custom Tools

Register Rust functions as custom tools:

```rust
use antigravity_sdk_rust::agent::{Agent, AgentConfig};
use antigravity_sdk_rust::tools::Tool;
use async_trait::async_trait;
use serde_json::Value;

struct WeatherTool;

#[async_trait]
impl Tool for WeatherTool {
    fn name(&self) -> String {
        "get_weather".to_string()
    }

    async fn execute(&self, args: Value) -> Result<Value, anyhow::Error> {
        let city = args.get("city").and_then(|c| c.as_str()).unwrap_or("Tokyo");
        Ok(serde_json::json!({ "weather": format!("It's sunny in {}", city) }))
    }
}
```

### Hooks and Policies

Control agent behavior with a declarative policy system:

```rust
use antigravity_sdk_rust::policy::{self, Policy};

let policies = vec![
    policy::deny_all(),                          // Block all tools by default
    policy::allow_creates(vec!["view_file"]),    // Allow reading files
];
```

## Architecture

For more information, see [ARCHITECTURE.md](file:///Volumes/goldcoders/antigravity-sdk-rust/ARCHITECTURE.md).

## License

[Apache License 2.0](LICENSE)
