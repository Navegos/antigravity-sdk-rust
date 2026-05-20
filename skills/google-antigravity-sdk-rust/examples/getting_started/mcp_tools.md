# Model Context Protocol (MCP) Integration

This example walkthrough outlines how the Google Antigravity Rust SDK integrates with external Model Context Protocol (MCP) servers.

For conceptual details and information on permissions, see the [MCP Integration Reference Guide](../../references/mcp_integration.md).

---

## Configuration Difference (Python vs Rust)

> [!IMPORTANT]
> In the Python SDK, you can programmatically define MCP servers directly in code using configuration structures passed to the config object (e.g. `mcp_servers=[McpStdioServer(...)]`).
>
> In the Rust SDK, MCP servers are **not configured programmatically in Rust code**. Instead, they are resolved globally by the local orchestration harness (`localharness`) from your global configuration file located at:
>
> `~/.gemini/antigravity/mcp_config.json`

### Declaring MCP Servers in `mcp_config.json`

To connect your agent to MCP servers, define them in the JSON file using standard formats. Below is an example config registering a local math server via `stdio` and a remote server via `sse`:

```json
{
  "mcpServers": {
    "math-server": {
      "command": "python3",
      "args": ["/path/to/math_server.py"]
    },
    "remote-sse-server": {
      "url": "https://example.com/mcp/sse",
      "headers": {
        "Authorization": "Bearer your-token-here"
      }
    }
  }
}
```

---

## Programmatic Verification from Rust

Once configured, your agent can discover and invoke the tools exposed by the MCP servers automatically. Simply initialize the agent normally:

```rust
use antigravity_sdk_rust::agent::{Agent, AgentConfig};
use antigravity_sdk_rust::policy;
use antigravity_sdk_rust::types::GeminiConfig;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let mut config = AgentConfig::default();
    
    // Safety policies to authorize tools
    config.policies = Some(vec![policy::allow_all()]);

    let mut agent = Agent::new(config);
    agent.start().await?;

    // The agent automatically leverages custom MCP tools registered in mcp_config.json
    let response = agent.chat("Use the math-server add_numbers tool to add 45 and 55.").await?;
    println!("Agent response: {}", response.text);

    agent.stop().await?;
    Ok(())
}
```
