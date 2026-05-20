use antigravity_sdk_rust::agent::{Agent, AgentConfig};
use antigravity_sdk_rust::policy;
use antigravity_sdk_rust::tools::Tool;
use antigravity_sdk_rust::types::GeminiConfig;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tracing_subscriber::EnvFilter;

struct LookupSkuTool;

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
                "fruit_name": {
                    "type": "string"
                }
            },
            "required": ["fruit_name"]
        }"#
    }

    async fn call(&self, args: Value) -> Result<Value, anyhow::Error> {
        let fruit_name = args
            .get("fruit_name")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("Missing fruit_name"))?
            .to_lowercase();

        let sku = match fruit_name.trim_end_matches('s') {
            "apple" => "SKU-APP-123",
            "banana" => "SKU-BAN-456",
            "orange" => "SKU-ORA-789",
            _ => "SKU-GEN-000",
        };

        Ok(Value::String(format!(
            "SKU for {} is {}. Order ID for restocking: ORD-{}-NEW",
            fruit_name, sku, sku
        )))
    }
}

struct RecordFruitTool {
    inventory: Arc<Mutex<HashMap<String, u32>>>,
}

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
                "sku": {
                    "type": "string"
                },
                "count": {
                    "type": "integer"
                }
            },
            "required": ["sku", "count"]
        }"#
    }

    async fn call(&self, args: Value) -> Result<Value, anyhow::Error> {
        let sku = args
            .get("sku")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("Missing sku"))?
            .to_string();

        let count = args
            .get("count")
            .and_then(Value::as_u64)
            .ok_or_else(|| anyhow::anyhow!("Missing count"))? as u32;

        let mut inv = self
            .inventory
            .lock()
            .map_err(|e| anyhow::anyhow!("Mutex lock poisoned: {}", e))?;
        let entry = inv.entry(sku.clone()).or_insert(0);
        *entry += count;
        let total = *entry;
        drop(inv);

        Ok(Value::String(format!(
            "Recorded {} units for {}. Total count is now {}.",
            count, sku, total
        )))
    }
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Initialize tracing subscriber
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    // Load environment variables from .env file if present
    dotenvy::dotenv().ok();

    let mut config = AgentConfig::default();

    if let Ok(harness_path) = std::env::var("ANTIGRAVITY_HARNESS_PATH") {
        config.binary_path = Some(harness_path);
    }

    let mut gemini_config = GeminiConfig::default();
    if let Ok(api_key) = std::env::var("GEMINI_API_KEY") {
        gemini_config.api_key = Some(api_key);
    }
    gemini_config.models.default.name = "gemini-3.5-flash".to_string();
    config.gemini_config = gemini_config;

    config.system_instructions = Some(antigravity_sdk_rust::types::SystemInstructions::Custom(
        antigravity_sdk_rust::types::CustomSystemInstructions {
            text:
                "You keep track of fruit inventory. To record fruits, you MUST first look up the \
                   fruit's SKU using lookup_fruit_sku, and then use that SKU with record_fruit."
                    .to_string(),
        },
    ));

    // Initialize shared mutable state for stateful tool
    let inventory = Arc::new(Mutex::new(HashMap::new()));

    // Register our custom tools
    config.tools = vec![
        Arc::new(LookupSkuTool),
        Arc::new(RecordFruitTool {
            inventory: inventory.clone(),
        }),
    ];

    // Restrict agent to ONLY these tools
    config.policies = Some(vec![
        policy::deny_all(),
        policy::allow("lookup_fruit_sku"),
        policy::allow("record_fruit"),
    ]);

    let mut agent = Agent::new(config);
    println!("Starting agent...");
    agent.start().await?;

    println!("  === Custom Tools Demo ===");

    // Turn 1: Lookup fruit SKU
    let prompt1 = "What is the SKU for apples? We need to order more.";
    println!("\n  User: {}", prompt1);
    let response1 = agent.chat(prompt1).await?;
    println!("  Agent: {}", response1.text);

    // Stateful interaction: record fruits across multiple turns
    println!("\n  === Stateful Tool (Fruit Counter) Demo ===");

    let turns = vec![
        "I have 5 apples.",
        "And I just got 3 bananas.",
        "Oh, and another 2 apples.",
    ];

    for user_input in turns {
        println!("\n  User: {}", user_input);
        let response = agent.chat(user_input).await?;
        println!("  Agent: {}", response.text);
    }

    agent.stop().await?;
    Ok(())
}
