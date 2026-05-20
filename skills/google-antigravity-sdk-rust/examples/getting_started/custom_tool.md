# Custom Tool Example

This example demonstrates how to equip an agent with custom capabilities by implementing the `Tool` trait.

## Defining and Registering a Custom Tool

In the Rust SDK, custom tools are defined by implementing the `Tool` trait. You must supply a JSON schema declaration representing the arguments the tool accepts:

```rust
use antigravity_sdk_rust::agent::{Agent, AgentConfig};
use antigravity_sdk_rust::policy;
use antigravity_sdk_rust::tools::Tool;
use antigravity_sdk_rust::types::GeminiConfig;
use serde_json::Value;
use std::sync::Arc;

// 1. Define the custom tool struct
pub struct WeatherTool;

impl Tool for WeatherTool {
    fn name(&self) -> &str {
        "get_current_temperature"
    }

    fn description(&self) -> &str {
        "Gets the current temperature for a given location."
    }

    fn parameters_json_schema(&self) -> &str {
        r#"{
            "type": "object",
            "properties": {
                "location": {
                    "type": "string",
                    "description": "The city and state, e.g. 'San Francisco, CA'"
                }
            },
            "required": ["location"]
        }"#
    }

    fn call(&self, args: Value) -> impl std::future::Future<Output = Result<Value, anyhow::Error>> + Send {
        async move {
            let location = args
                .get("location")
                .and_then(Value::as_str)
                .ok_or_else(|| anyhow::anyhow!("Missing 'location' parameter"))?;
            
            // In a real application, call an external weather API here
            let temperature = format!("The temperature in {} is 72°F.", location);
            Ok(Value::String(temperature))
        }
    }
}

// 2. Configure the agent with the custom tool
#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let mut config = AgentConfig::default();
    
    // Add custom tool
    config.tools = vec![Arc::new(WeatherTool)];
    config.policies = Some(vec![policy::allow_all()]);

    let mut agent = Agent::new(config);
    agent.start().await?;

    let response = agent.chat("What is the temperature in Mountain View?").await?;
    println!("Agent response: {}", response.text);

    agent.stop().await?;
    Ok(())
}
```

---

## Maintaining State Thread-Safely in Rust

> [!NOTE]
> In the Python SDK, state is maintained via a `ToolContext` instance injected into the tool function. In the Rust SDK, since tool definitions must be `Send + Sync`, you achieve state retention using standard thread-safe interior mutability wrappers (like `std::sync::Mutex` or `tokio::sync::Mutex` inside your tool struct).

Here is how to implement a stateful fruit count tracker tool:

```rust
use antigravity_sdk_rust::agent::{Agent, AgentConfig};
use antigravity_sdk_rust::policy;
use antigravity_sdk_rust::tools::Tool;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub struct FruitInventoryTool {
    // Thread-safe inventory state
    inventory: Mutex<HashMap<String, i32>>,
}

impl FruitInventoryTool {
    pub fn new() -> Self {
        Self {
            inventory: Mutex::new(HashMap::new()),
        }
    }
}

impl Tool for FruitInventoryTool {
    fn name(&self) -> &str {
        "record_fruit"
    }

    fn description(&self) -> &str {
        "Records the mention of fruits and updates the total count."
    }

    fn parameters_json_schema(&self) -> &str {
        r#"{
            "type": "object",
            "properties": {
                "fruit_name": {
                    "type": "string",
                    "description": "The name of the fruit."
                },
                "count": {
                    "type": "integer",
                    "description": "The number of fruits mentioned."
                }
            },
            "required": ["fruit_name", "count"]
        }"#
    }

    fn call(&self, args: Value) -> impl std::future::Future<Output = Result<Value, anyhow::Error>> + Send {
        async move {
            let fruit_name = args
                .get("fruit_name")
                .and_then(Value::as_str)
                .ok_or_else(|| anyhow::anyhow!("Missing 'fruit_name'"))?;
            let count = args
                .get("count")
                .and_then(Value::as_i64)
                .ok_or_else(|| anyhow::anyhow!("Missing 'count'"))? as i32;

            let mut inv = self
                .inventory
                .lock()
                .map_err(|e| anyhow::anyhow!("Mutex lock poisoned: {}", e))?;
            let entry = inv.entry(fruit_name.to_string()).or_insert(0);
            *entry += count;

            let result_msg = format!(
                "Recorded {} {}(s). Total {} count is now {}.",
                count, fruit_name, fruit_name, *entry
            );
            Ok(Value::String(result_msg))
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let mut config = AgentConfig::default();
    config.tools = vec![Arc::new(FruitInventoryTool::new())];
    config.policies = Some(vec![policy::allow_all()]);
    config.system_instructions = Some(antigravity_sdk_rust::types::SystemInstructions::Custom(
        antigravity_sdk_rust::types::CustomSystemInstructions {
            text: "You are a fruit inventory assistant. Use the record_fruit tool to record fruits mentioned by the user.".to_string(),
        }
    ));

    let mut agent = Agent::new(config);
    agent.start().await?;

    // Turn 1
    println!("User: I have 5 apples.");
    let res1 = agent.chat("I have 5 apples.").await?;
    println!("Agent: {}", res1.text);

    // Turn 2
    println!("User: I just got 3 more apples.");
    let res2 = agent.chat("I just got 3 more apples.").await?;
    println!("Agent: {}", res2.text);

    agent.stop().await?;
    Ok(())
}
```
