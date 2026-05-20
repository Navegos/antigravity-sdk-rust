# System Instructions and Persona

This example demonstrates how to configure system instructions or persona for an agent using the Google Antigravity Rust SDK. You can either append structured guidelines or completely overwrite the default instructions.

## Appending to Instructions (Recommended)

To append custom instructions, behaviors, or titles while retaining the SDK's built-in safety rules and core operational protocols, use `AppendedSystemInstructions`.

```rust
use antigravity_sdk_rust::agent::{Agent, AgentConfig};
use antigravity_sdk_rust::policy;
use antigravity_sdk_rust::types::{
    AppendedSystemInstructions, SystemInstructionSection, SystemInstructions,
};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let mut config = AgentConfig::default();
    config.policies = Some(vec![policy::allow_all()]);

    // Define the appended structured system instructions
    let appended_si = AppendedSystemInstructions {
        // Sets the core personality of the agent
        custom_identity: Some("You are a helpful software agent who talks like a pirate.".to_string()),
        // Appends specific markdown sections of instructions
        appended_sections: vec![
            SystemInstructionSection {
                title: "Vocabulary Guide".to_string(),
                content: "Always use phrases like 'Ahoy!', 'Matey!', or 'Shiver me timbers!' in your responses.".to_string(),
            }
        ],
    };

    config.system_instructions = Some(SystemInstructions::Appended(appended_si));

    let mut agent = Agent::new(config);
    agent.start().await?;

    let response = agent.chat("Hello! Who are you?").await?;
    println!("Agent response: {}", response.text);

    agent.stop().await?;
    Ok(())
}
```

---

## Overwriting Instructions (Advanced)

To completely replace all default instructions (including safety mandates and system prompts) with your own custom instructions text, use `CustomSystemInstructions`.

> [!WARNING]
> Use this with caution. By doing so, you bypass all default helper guidelines and are responsible for specifying all necessary formatting rules, safety measures, and operational constraints yourself.

```rust
use antigravity_sdk_rust::agent::{Agent, AgentConfig};
use antigravity_sdk_rust::policy;
use antigravity_sdk_rust::types::{CustomSystemInstructions, SystemInstructions};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let mut config = AgentConfig::default();
    config.policies = Some(vec![policy::allow_all()]);

    let custom_si = CustomSystemInstructions {
        text: "You are a minimal assistant. You only answer questions with 'Yes' or 'No'.".to_string(),
    };

    config.system_instructions = Some(SystemInstructions::Custom(custom_si));

    let mut agent = Agent::new(config);
    agent.start().await?;

    let response = agent.chat("Is the sky blue?").await?;
    println!("Agent: {}", response.text);

    agent.stop().await?;
    Ok(())
}
```
