# Triggers and Periodic Checks

This example walkthrough demonstrates how to build and register background triggers in the Google Antigravity Rust SDK. Triggers are async tasks implementing the `Trigger` trait that run in the background to monitor external events (e.g. timers, files, webhooks) and dispatch notifications to the agent.

## Code Example

```rust
use antigravity_sdk_rust::agent::{Agent, AgentConfig};
use antigravity_sdk_rust::connection::Connection;
use antigravity_sdk_rust::policy;
use antigravity_sdk_rust::triggers::Trigger;
use antigravity_sdk_rust::types::GeminiConfig;
use async_trait::async_trait;
use std::sync::Arc;
use std::time::Duration;

// =============================================================================
// 1. Periodic Trigger Struct
// =============================================================================
struct HeartbeatTrigger {
    interval_secs: u64,
}

#[async_trait]
impl Trigger for HeartbeatTrigger {
    async fn run(&self, connection: Arc<dyn Connection>) -> Result<(), anyhow::Error> {
        println!("[Trigger] Starting heartbeat loop...");
        loop {
            tokio::time::sleep(Duration::from_secs(self.interval_secs)).await;
            
            println!("[Trigger] Firing heartbeat event!");
            // Dispatch a notification event to the agent session
            connection
                .send_trigger_notification("System notification: Status is OK")
                .await?;
        }
    }
}

// =============================================================================
// 2. Custom Web Polling Trigger
// =============================================================================
struct CustomPollTrigger;

#[async_trait]
impl Trigger for CustomPollTrigger {
    async fn run(&self, connection: Arc<dyn Connection>) -> Result<(), anyhow::Error> {
        println!("[Trigger] Custom poll worker started.");
        
        // Simulating single event check
        tokio::time::sleep(Duration::from_secs(10)).await;
        
        connection
            .send_trigger_notification("External alert: High memory usage detected!")
            .await?;
            
        Ok(())
    }
}

// =============================================================================
// Configuration and Registration
// =============================================================================
#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let mut config = AgentConfig::default();
    config.policies = Some(vec![policy::allow_all()]);

    // Register triggers inside configuration
    config.triggers = vec![
        Arc::new(HeartbeatTrigger { interval_secs: 5 }),
        Arc::new(CustomPollTrigger),
    ];

    let mut agent = Agent::new(config);
    agent.start().await?;

    // Perform regular chat turns. Registered triggers execute concurrently in the background.
    let response = agent.chat("Keep an eye on system health alerts.").await?;
    println!("Agent response: {}", response.text);

    // Keep session alive to observe triggers
    tokio::time::sleep(Duration::from_secs(12)).await;

    agent.stop().await?;
    Ok(())
}
```

---

## Detailed Explanation

1. **Trigger Trait implementation**:
   * Implement `async_trait` for the `Trigger` trait.
   * Inside `run`, utilize the provided `Arc<dyn Connection>` parameter.
2. **Sending Notifications**:
   * Call `connection.send_trigger_notification(message)` to send background alerts or information updates to the agent session.
3. **Threading & Lifecycle**:
   * The `Agent` starts and spawns all registered triggers automatically into individual `tokio::spawn` background workers when `agent.start()` is invoked. They run concurrently with standard user chat interactions.
