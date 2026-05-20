# Custom Tools Example Walkthrough

This walkthrough demonstrates how to define, register, and execute custom Rust tools.

## Example Source Code

The full implementation is located in [examples/custom_tools.rs](file:///Volumes/goldcoders/antigravity-sdk-rust/examples/custom_tools.rs). Below is the code detailing custom tool trait implementation and shared state execution:

```rust
use antigravity_sdk_rust::agent::{Agent, AgentConfig};
use antigravity_sdk_rust::policy;
use antigravity_sdk_rust::tools::Tool;
use antigravity_sdk_rust::types::GeminiConfig;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// 1. Stateless Tool
struct LookupSkuTool;

#[async_trait]
impl Tool for LookupSkuTool {
    fn name(&self) -> &'static str {
        "lookup_fruit_sku"
    }

    fn description(&self) -> &'static str {
        "Looks up the SKU for a given fruit."
    }

    fn parameters_json_schema(&self) -> &'static str {
        r#"{
            "type": "object",
            "properties": {
                "fruit_name": { "type": "string" }
            },
            "required": ["fruit_name"]
        }"#
    }

    async fn call(&self, args: Value) -> Result<Value, anyhow::Error> {
        let fruit_name = args.get("fruit_name").and_then(Value::as_str).unwrap_or("");
        let sku = match fruit_name.to_lowercase().as_str() {
            "apple" => "SKU-APP-123",
            "banana" => "SKU-BAN-456",
            _ => "SKU-GEN-000",
        };
        Ok(Value::String(sku.to_string()))
    }
}

// 2. Stateful Tool
struct RecordFruitTool {
    inventory: Arc<Mutex<HashMap<String, u32>>>,
}

#[async_trait]
impl Tool for RecordFruitTool {
    fn name(&self) -> &'static str {
        "record_fruit"
    }

    fn description(&self) -> &'static str {
        "Records the count of fruits by SKU."
    }

    fn parameters_json_schema(&self) -> &'static str {
        r#"{
            "type": "object",
            "properties": {
                "sku": { "type": "string" },
                "count": { "type": "integer" }
            },
            "required": ["sku", "count"]
        }"#
    }

    async fn call(&self, args: Value) -> Result<Value, anyhow::Error> {
        let sku = args.get("sku").and_then(Value::as_str).unwrap().to_string();
        let count = args.get("count").and_then(Value::as_u64).unwrap() as u32;

        let mut inv = self.inventory.lock().unwrap();
        let entry = inv.entry(sku.clone()).or_insert(0);
        *entry += count;
        let total = *entry;

        Ok(Value::String(format!("Total for {} is now {}", sku, total)))
    }
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let mut config = AgentConfig::default();
    
    // Configure Gemini
    let mut gemini_config = GeminiConfig::default();
    gemini_config.models.default.name = "gemini-3.5-flash".to_string();
    config.gemini_config = gemini_config;

    config.system_instructions = Some(antigravity_sdk_rust::types::SystemInstructions::Custom(
        antigravity_sdk_rust::types::CustomSystemInstructions {
            text: "You record fruit inventory. First lookup the SKU, then record it.".to_string(),
        },
    ));

    // Initialize shared state
    let inventory = Arc::new(Mutex::new(HashMap::new()));

    // Register Tools
    config.tools = vec![
        Arc::new(LookupSkuTool),
        Arc::new(RecordFruitTool { inventory: inventory.clone() }),
    ];

    // Deny-by-default safety policy
    config.policies = Some(vec![
        policy::deny_all(),
        policy::allow("lookup_fruit_sku"),
        policy::allow("record_fruit"),
    ]);

    let mut agent = Agent::new(config);
    agent.start().await?;

    let response = agent.chat("I have 5 apples. Record them.").await?;
    println!("Agent: {}", response.text);

    agent.stop().await?;
    Ok(())
}
```

---

## Key Takeaways

* **Implementing custom tools**: You create a struct and implement the asynchronous `Tool` trait, specifying `name()`, `description()`, parameters schema, and `call()`.
* **Stateful tools**: By passing a thread-safe reference-counted pointer (`Arc<Mutex<T>>`) to the tool structure, you can mutate local state during agent turns.
* **Deny-by-default policy posture**: Under this security layout, the agent is restricted to calling *only* the whitelisted tool names.

---

## How to Run

```bash
export GEMINI_API_KEY="your_api_key_here"
cargo run --example custom_tools
```
