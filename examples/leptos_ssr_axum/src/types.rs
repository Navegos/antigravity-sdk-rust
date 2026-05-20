use serde::{Deserialize, Serialize};

/// Represents a tool call recorded during the message generation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClientToolCall {
    pub name: String,
    pub args: String, // Stringified JSON args for simpler serialization and rendering
}

/// A single chat message exchanged between user and assistant.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChatMessage {
    /// Unique message identifier (monotonically increasing per session)
    #[serde(default)]
    pub id: u64,
    /// "user" or "assistant"
    pub role: String,
    /// The message content (plain text or markdown)
    pub content: String,
    /// Unix timestamp in seconds
    pub timestamp: u64,
    /// Optional thinking/reasoning text
    #[serde(default)]
    pub thinking: Option<String>,
    /// Optional list of tool calls made
    #[serde(default)]
    pub tool_calls: Option<Vec<ClientToolCall>>,
}
