use antigravity_sdk_rust::agent::Agent;
use antigravity_sdk_rust::policy;
use antigravity_sdk_rust::tools::Tool;
use serde_json::Value;
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

struct FetchNotesTool;

impl Tool for FetchNotesTool {
    fn name(&self) -> &'static str {
        "fetch_unstructured_meeting_notes"
    }

    fn description(&self) -> &'static str {
        "Retrieves the raw unstructured notes for a given meeting ID."
    }

    fn parameters_json_schema(&self) -> &'static str {
        r#"{
            "type": "object",
            "properties": {
                "meeting_id": {
                    "type": "string"
                }
            },
            "required": ["meeting_id"]
        }"#
    }

    async fn call(&self, args: Value) -> Result<Value, anyhow::Error> {
        let meeting_id = args
            .get("meeting_id")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("Missing meeting_id"))?;

        if meeting_id == "meeting-2026-05" {
            Ok(Value::String(
                "Discussed launch timeline for project X. Alice agreed to update the textproto tests by Monday. \
                 Bob mentioned he will run the final E2E benchmarks tomorrow. I will push the release build \
                 once the tests are green."
                    .to_string(),
            ))
        } else {
            Ok(Value::String("Error: Meeting notes not found.".to_string()))
        }
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

    let harness_path = std::env::var("ANTIGRAVITY_HARNESS_PATH").ok();
    let api_key = std::env::var("GEMINI_API_KEY").ok();

    let mut builder = Agent::builder();
    if let Some(path) = harness_path {
        builder = builder.binary_path(path);
    }
    if let Some(key) = api_key {
        builder = builder.api_key(key);
    }

    // Define response schema for meeting summaries and action items
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

    let agent = builder
        .default_model("gemini-3.5-flash")
        .response_schema(response_schema.to_string())
        .tool(Arc::new(FetchNotesTool))
        .policy(policy::deny_all())
        .policy(policy::allow("fetch_unstructured_meeting_notes"))
        .build();

    println!("Starting agent...");
    let agent = agent.start().await?;

    let prompt = "Use the fetch_unstructured_meeting_notes tool to retrieve notes for \
                  'meeting-2026-05' and return the meeting summary with the appropriate \
                  action item list. Ensure each action item includes 'assignee', \
                  'task', and 'deadline'.";

    println!("\n  Sending prompt to agent...\n  {}", prompt);
    let response = agent.chat(prompt).await?;

    println!("\n  Extracting structured meeting action items...");

    // Find the step that contains structured_output
    let mut found = false;
    for step in &response.steps {
        if let Some(ref structured) = step.structured_output {
            println!("\n  === Structured Meeting Action Items ===");
            if let Some(items) = structured.get("action_items").and_then(Value::as_array) {
                for item in items {
                    println!(
                        "  - Assignee: {:?}",
                        item.get("assignee").and_then(Value::as_str).unwrap_or("")
                    );
                    println!(
                        "    Task:     {:?}",
                        item.get("task").and_then(Value::as_str).unwrap_or("")
                    );
                    println!(
                        "    Deadline: {:?}",
                        item.get("deadline").and_then(Value::as_str).unwrap_or("")
                    );
                    println!();
                }
            } else {
                println!("No action_items field or not an array: {:?}", structured);
            }
            found = true;
            break;
        }
    }

    if !found {
        println!("\n  Failed to extract structured summary natively.");
        println!("  Final Text Response: {}", response.text);
    }

    agent.stop().await?;
    Ok(())
}
