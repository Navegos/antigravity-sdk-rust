//! Connection abstractions for communication with the agent execution harness.
//!
//! This module defines the [`Connection`] trait, which provides a transport-agnostic interface
//! for sending prompts, receiving step updates, and executing tool and question handshakes.

use crate::types::{QuestionHookResult, Step, ToolResult};
use async_trait::async_trait;
use futures_util::stream::BoxStream;

/// Trait representing an active session connection with the agentic harness.
///
/// A connection manages process or network lifecycle, handshaking, and event streaming.
/// Common implementations include [`LocalConnection`](crate::local::LocalConnection) which spawns
/// a local helper subprocess and upgrades communication to a WebSocket protocol.
#[async_trait]
pub trait Connection: Send + Sync {
    /// Returns the conversation ID for the connection.
    fn conversation_id(&self) -> &str;

    /// Returns whether the connection is currently idle.
    fn is_idle(&self) -> bool;

    /// Subscribes to the stream of step updates from the connection.
    fn receive_steps(&self) -> BoxStream<'static, Result<Step, anyhow::Error>>;

    /// Sends a standard user text prompt to the connection.
    async fn send(&self, content: &str) -> Result<(), anyhow::Error>;

    /// Sends a trigger notification message to the connection.
    async fn send_trigger_notification(&self, content: &str) -> Result<(), anyhow::Error>;

    /// Sends a halt/cancellation request to the connection.
    async fn send_halt_request(&self) -> Result<(), anyhow::Error>;

    /// Sends approval/rejection for a tool execution confirmation request.
    async fn send_tool_confirmation(
        &self,
        trajectory_id: &str,
        step_index: u32,
        accepted: bool,
    ) -> Result<(), anyhow::Error>;

    /// Sends the result of a tool execution back to the connection.
    async fn send_tool_response(&self, id: &str, result: ToolResult) -> Result<(), anyhow::Error>;

    /// Sends the answers to user questions back to the connection.
    async fn send_question_response(
        &self,
        trajectory_id: &str,
        step_index: u32,
        answers: QuestionHookResult,
    ) -> Result<(), anyhow::Error>;

    /// Gracefully closes the connection.
    async fn disconnect(&self) -> Result<(), anyhow::Error>;
}
