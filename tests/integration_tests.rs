#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::field_reassign_with_default
)]

use antigravity_sdk_rust::agent::{Agent, AgentConfig};
use antigravity_sdk_rust::policy;
use antigravity_sdk_rust::types::{
    BuiltinTools, CapabilitiesConfig, GeminiConfig, GenerationConfig, ModelConfig, ModelEntry,
};

#[tokio::test]
async fn test_agent_chat_integration() {
    let _ = tracing_subscriber::fmt::try_init();

    let mut config = AgentConfig::default();

    // Set up mock harness path (absolute path)
    let harness_path = std::env::current_dir()
        .unwrap()
        .join("tests/mock_localharness.py")
        .to_string_lossy()
        .into_owned();

    config.binary_path = Some(harness_path);
    config.gemini_config = GeminiConfig {
        api_key: Some("test_api_key".to_string()),
        models: ModelConfig {
            default: ModelEntry {
                name: "gemini-2.5-flash".to_string(),
                api_key: None,
                generation: GenerationConfig {
                    thinking_level: None,
                },
            },
            image_generation: ModelEntry::default(),
        },
    };

    // Disable write tools to avoid policy assertion issues, or register allow_all policy
    config.capabilities = CapabilitiesConfig {
        enabled_tools: Some(vec![BuiltinTools::ViewFile]), // Only read-only
        disabled_tools: None,
        compaction_threshold: None,
        image_model: None,
        finish_tool_schema_json: None,
    };

    config.policies = Some(vec![policy::allow_all()]);
    config.conversation_id = Some("test_conv_123".to_string());
    config.workspaces = Some(vec![
        std::env::current_dir()
            .unwrap()
            .to_string_lossy()
            .into_owned(),
    ]);

    let mut agent = Agent::new(config);

    // 1. Start agent
    agent.start().await.expect("Failed to start agent");

    // 2. Chat with agent
    let response = agent
        .chat("hello")
        .await
        .expect("Failed to chat with agent");

    // 3. Verify response
    assert_eq!(
        response.text,
        "Hello from mock harness!How can I help you today?"
    );
    assert_eq!(response.steps.len(), 2);

    // 4. Verify conversation metadata
    let conversation = agent.conversation().expect("Failed to get conversation");
    assert_eq!(conversation.conversation_id(), "test_conv_123");

    // 5. Stop agent
    agent.stop().await.expect("Failed to stop agent");
}

#[tokio::test]
async fn test_agent_start_mutually_exclusive_capabilities() {
    let mut config = AgentConfig::default();
    config.binary_path = Some("some_path".to_string());
    config.capabilities = CapabilitiesConfig {
        enabled_tools: Some(vec![BuiltinTools::ViewFile]),
        disabled_tools: Some(vec![BuiltinTools::RunCommand]),
        compaction_threshold: None,
        image_model: None,
        finish_tool_schema_json: None,
    };

    let mut agent = Agent::new(config);
    let result = agent.start().await;
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("mutually exclusive"));
}
