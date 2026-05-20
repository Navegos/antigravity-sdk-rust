use serde::{Deserialize, Serialize};

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
}
