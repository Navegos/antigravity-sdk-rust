# Google Antigravity Rust SDK

[![CI](https://github.com/codeitlikemiley/antigravity-sdk-rust/actions/workflows/ci.yml/badge.svg)](https://github.com/codeitlikemiley/antigravity-sdk-rust/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/antigravity-sdk-rust.svg)](https://crates.io/crates/antigravity-sdk-rust)
[![Docs.rs](https://docs.rs/antigravity-sdk-rust/badge.svg)](https://docs.rs/antigravity-sdk-rust)
[![Rust](https://img.shields.io/badge/rust-2024-blue.svg)](https://www.rust-lang.org)

The Google Antigravity SDK is a Rust library for building AI agents powered by Antigravity and Gemini. It provides a secure, scalable, and stateful infrastructure layer that abstracts the agentic loop, letting you focus on what your agent *does* rather than how it runs.

## Installation

1. Add the SDK and tokio to your project:
   ```sh
   cargo add antigravity-sdk-rust
   cargo add tokio --features full
   ```

2. **Obtain the `localharness` binary:**
   The SDK relies on a compiled native Go runtime binary (`localharness`) to orchestrate agent operations. You have two options to install it:

   - **Option A: Without Python/pip (Recommended for Rust-only environments & web servers)**
     Run the helper installer script to download and extract the binary directly from PyPI (wheels are standard ZIP files):
     ```sh
     ./scripts/install_harness.sh
     export ANTIGRAVITY_HARNESS_PATH="$(pwd)/bin/localharness"
     ```

   - **Option B: Using Python/pip**
     If Python is already installed on your development machine, simply run:
     ```sh
     pip install google-antigravity
     ```
     The Rust SDK will automatically locate the binary inside the Python package installation directory fallback.

## Quickstart

Get started by setting your API key and running the `hello_world` example:

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
    fn name(&self) -> &str {
        "get_weather"
    }

    fn description(&self) -> &str {
        "Get the current weather for a city."
    }

    fn parameters_json_schema(&self) -> &str {
        r#"{
            "type": "object",
            "properties": {
                "city": {
                    "type": "string"
                }
            },
            "required": ["city"]
        }"#
    }

    async fn call(&self, args: Value) -> Result<Value, anyhow::Error> {
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
    policy::allow("VIEW_FILE"),                  // Allow reading/viewing files
];
```

## Web Integration

The SDK provides two patterns for building web applications with AI agents:

### Leptos + Axum (Native)

For standard web servers (VPS, Docker, bare metal). The Agent runs in-process with full SDK features.

```sh
cd examples/leptos_axum
echo "GEMINI_API_KEY=your-key" > .env
cargo leptos serve
# Open http://localhost:3000
```

### Leptos + Spin/WASI (Edge)

For edge/serverless deployments on [Fermyon Spin](https://www.fermyon.com/spin). Since Spin components cannot make outbound TCP/WebSocket connections, two modes are supported:

**Sidecar Mode (Full SDK):**
```sh
# Terminal 1: Start the agent sidecar
cd examples/agent_server
GEMINI_API_KEY=your-key cargo run

# Terminal 2: Start Spin
cd examples/leptos_ssr_axum
spin build --up
# Open http://localhost:3000
```

**Direct Mode (Lightweight, no sidecar):**
```sh
cd examples/leptos_ssr_axum
spin build --up --variable gemini_api_key="your-key"
```

> See [`examples/README.md`](examples/README.md) for detailed architecture diagrams and configuration options.

### GeminiDirectClient

For environments where TCP/WebSocket is unavailable (e.g. WASI, embedded), the SDK provides a transport-agnostic Gemini API client that builds HTTP request payloads and parses responses — the caller provides the HTTP transport:

```rust
use antigravity_sdk_rust::direct::{GeminiDirectClient, ChatEntry};
use antigravity_sdk_rust::types::GeminiConfig;

let client = GeminiDirectClient::new(&GeminiConfig::default())
    .with_system_instruction("You are a helpful assistant.".to_string());

// Build the request (URL, headers, body) — send via your HTTP client
let request = client.build_request("your-api-key", "Hello!", &[]).unwrap();

// Parse the response
let text = GeminiDirectClient::parse_response(&response_bytes).unwrap();
```

## Local Development

This project uses [just](https://github.com/casey/just) to manage development tasks.

- **Check Code Quality**: Run all style, lint, and test checks.
  ```sh
  just check
  ```
- **Install Local Harness**: Download and configure the required `localharness` binary.
  ```sh
  just install
  ```
- **Bump version & Release tag**: Bump the package version, update Cargo.lock, commit using `--no-verify` (skipping git hooks), and tag the release.
  ```sh
  # Auto-bump patch version (e.g., 0.1.0 -> 0.1.1)
  just version
  
  # Or force/override with a specific version
  just version 0.2.0
  ```
- **Publish to Crates.io**: Manually publish the package to crates.io (runs `just check` first).
  ```sh
  just publish
  ```

## Architecture

For more information, see [ARCHITECTURE.md](ARCHITECTURE.md).

## License

This project is licensed under the [MIT License](LICENSE).
