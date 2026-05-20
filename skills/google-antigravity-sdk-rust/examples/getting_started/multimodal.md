# Multimodal Examples

This example demonstrates how to configure and use multimodal capabilities in the Google Antigravity Rust SDK.

## Multimodal Outputs (Generating Images)

To enable the agent to generate images on request, you must add `BuiltinTools::GenerateImage` to your agent's enabled tools capability list:

```rust
use antigravity_sdk_rust::agent::{Agent, AgentConfig};
use antigravity_sdk_rust::policy;
use antigravity_sdk_rust::types::{BuiltinTools, CapabilitiesConfig};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let mut config = AgentConfig::default();
    
    // Enable the GENERATE_IMAGE tool in capabilities
    let mut capabilities = CapabilitiesConfig::default();
    capabilities.enabled_tools = Some(vec![BuiltinTools::GenerateImage]);
    config.capabilities = capabilities;

    // A safety policy is required for write tools
    config.policies = Some(vec![policy::allow_all()]);

    config.system_instructions = Some(antigravity_sdk_rust::types::SystemInstructions::Custom(
        antigravity_sdk_rust::types::CustomSystemInstructions {
            text: format!(
                "You are an artist assistant. You have access to the '{}' tool. Use it when asked to generate or draw images.",
                BuiltinTools::GenerateImage.as_str()
            ),
        }
    ));

    let mut agent = Agent::new(config);
    agent.start().await?;

    let response = agent.chat("Generate an image of a futuristic floating island.").await?;
    println!("Agent response: {}", response.text);

    agent.stop().await?;
    Ok(())
}
```

---

## Multimodal Inputs (Differences with Python SDK)

> [!WARNING]
> In the Python SDK, you can programmatically construct `Image` and `Document` input wrappers and pass them as a list inside the `chat([prompt, image])` invocation.
>
> In the Rust SDK, the `agent.chat(prompt: &str)` method accepts a single string slice and does not support passing raw image or document structs programmatically. If you need the agent to perform vision analysis, place the image files in the agent's active sandbox workspace or app data directory and refer to them by path in your text prompt. The model will automatically resolve and parse files located in its workspace during execution.
