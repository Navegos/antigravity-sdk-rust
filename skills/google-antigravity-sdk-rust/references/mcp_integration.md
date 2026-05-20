# Custom Tools & MCP Integration

This guide details how to extend the agent's capabilities by writing custom Rust tools or integrating Model Context Protocol (MCP) servers.

## Registering Custom Tools in Rust

To provide the model with a custom function or action, you must implement the `Tool` trait and register it inside the `AgentConfig`.

### The Tool Trait Definition

```rust
use serde_json::Value;

pub trait Tool: Send + Sync {
    /// The name of the tool exposed to the model (e.g. "my_custom_action").
    fn name(&self) -> &str;

    /// A clear description explaining what the tool does and when the model should invoke it.
    fn description(&self) -> &str;

    /// A JSON Schema string specifying the parameters required by the tool.
    fn parameters_json_schema(&self) -> &str;

    /// The asynchronous function executed when the model calls the tool.
    /// Receives arguments as a JSON `Value` and returns a JSON `Value` result.
    async fn call(&self, args: Value) -> Result<Value, anyhow::Error>;
}
```

---

## Implementing a Custom Tool

Below is an implementation of a tool that calculates the hash of a string:

```rust
use antigravity_sdk_rust::agent::{Agent, AgentConfig};
use antigravity_sdk_rust::tools::Tool;
use serde_json::Value;
use sha2::{Sha256, Digest};
use std::sync::Arc;

struct HashStringTool;

impl Tool for HashStringTool {
    fn name(&self) -> &str {
        "hash_string"
    }

    fn description(&self) -> &str {
        "Computes the SHA-256 hash of a provided input text."
    }

    fn parameters_json_schema(&self) -> &str {
        r#"{
            "type": "object",
            "properties": {
                "text": {
                    "type": "string",
                    "description": "The string to hash"
                }
            },
            "required": ["text"]
        }"#
    }

    async fn call(&self, args: Value) -> Result<Value, anyhow::Error> {
        let text = args.get("text")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("Missing parameter: text"))?;
        
        let mut hasher = Sha256::new();
        hasher.update(text.as_bytes());
        let result = format!("{:x}", hasher.finalize());

        Ok(serde_json::json!({ "sha256": result }))
    }
}
```

---

## Registering and Allowing Tools

Custom tools must be added to the `tools` array of `AgentConfig`. If using a Deny-by-Default policy structure, they must also be explicitly whitelisted:

```rust
use antigravity_sdk_rust::policy;

let mut config = AgentConfig::default();

// Add tool implementation
config.tools = vec![Arc::new(HashStringTool)];

// Restrict agent to ONLY this tool
config.policies = Some(vec![
    policy::deny_all(),
    policy::allow("hash_string"),
]);

let mut agent = Agent::new(config);
agent.start().await?;
```

---

## Model Context Protocol (MCP) Servers

The `localharness` subprocess natively supports routing tool calls to external Model Context Protocol (MCP) servers.

* **Configuration**: MCP servers are declared in the host environment configuration (typically under `~/.gemini/antigravity/mcp_config.json`).
* **Tool Discovery**: When the agent starts, the `localharness` connects to these MCP servers, queries their available tools, merges them with custom Rust tools, and exposes them dynamically to the Gemini model.
* **Resolution**: When a tool call is requested by the model, the harness determines if it maps to a built-in tool, a custom Rust tool (forwarded via WebSocket to the SDK), or an MCP server tool (invoked directly by the harness).
