use serde::{Deserialize, Serialize};

/// A single chat message exchanged between user and assistant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// "user" or "assistant"
    pub role: String,
    /// The message content (plain text or markdown)
    pub content: String,
    /// Unix timestamp in seconds
    pub timestamp: u64,
}
