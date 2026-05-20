# Structured Output Example

This example walkthrough demonstrates how to configure the agent to respond strictly with valid JSON conforming to a specified JSON Schema.

## Code Example

The full implementation is located in [examples/structured_output.rs](file:///Volumes/goldcoders/antigravity-sdk-rust/examples/structured_output.rs):

```rust
use antigravity_sdk_rust::agent::{Agent, AgentConfig};
use antigravity_sdk_rust::policy;
use antigravity_sdk_rust::types::GeminiConfig;
use serde_json::Value;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let mut config = AgentConfig::default();
    
    let mut gemini_config = GeminiConfig::default();
    gemini_config.models.default.name = "gemini-3.5-flash".to_string();
    config.gemini_config = gemini_config;

    // 1. Declare JSON Schema for model response
    let response_schema = r#"{
        "type": "object",
        "properties": {
            "action_items": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "assignee": { "type": "string" },
                        "task": { "type": "string" },
                        "deadline": { "type": "string" }
                    },
                    "required": ["assignee", "task", "deadline"]
                }
            }
        },
        "required": ["action_items"]
    }"#;
    config.response_schema = Some(response_schema.to_string());

    let mut agent = Agent::new(config);
    agent.start().await?;

    let prompt = "Extract action items from: 'Alice needs to fix tests by Monday, Bob will benchmark tomorrow.'";
    let response = agent.chat(prompt).await?;

    // 2. Extract structured JSON object from Step history
    for step in &response.steps {
        if let Some(ref structured) = step.structured_output {
            println!("=== Extracted Action Items ===");
            if let Some(items) = structured.get("action_items").and_then(Value::as_array) {
                for item in items {
                    println!("- Task: {:?}", item.get("task"));
                    println!("  Assignee: {:?}", item.get("assignee"));
                }
            }
        }
    }

    agent.stop().await?;
    Ok(())
}
```

---

## Detailed Explanation

1. **`response_schema` Configuration**: Passing a JSON Schema forces the LLM's response generation parameters to structure the output in JSON format matching the schema rules.
2. **Accessing Output**:
   * The text response `response.text` contains raw text representation.
   * To retrieve the parsed JSON programmatically, iterate through `response.steps` and inspect the `structured_output` field (`Option<serde_json::Value>`).
   * The SDK automatically decodes the valid JSON payload emitted by the model during the conversation.

---

## How to Run

```bash
export GEMINI_API_KEY="your_api_key_here"
cargo run --example structured_output
```
