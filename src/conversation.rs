use crate::connection::Connection;
use crate::types::{
    ChatResponse, Step, StepSource, StepTarget, StepType, StreamChunk, UsageMetadata,
};
use futures_util::StreamExt;
use futures_util::stream::{self, BoxStream};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::Mutex;

const DEFAULT_MAX_HISTORY_SIZE: usize = 10_000;

#[derive(Debug)]
pub struct ConversationState {
    pub steps: Vec<Step>,
    pub turn_start_indices: Vec<usize>,
    pub compaction_indices: Vec<usize>,
    pub cumulative_usage: UsageMetadata,
    pub turn_usage: Option<UsageMetadata>,
}

pub struct Conversation {
    conn: Arc<dyn Connection>,
    max_history_size: usize,
    state: Arc<Mutex<ConversationState>>,
}

impl std::fmt::Debug for Conversation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Conversation")
            .field("conversation_id", &self.conversation_id())
            .field("max_history_size", &self.max_history_size)
            .finish_non_exhaustive()
    }
}

impl Conversation {
    pub fn new(conn: Arc<dyn Connection>, max_history_size: Option<usize>) -> Self {
        Self {
            conn,
            max_history_size: max_history_size.unwrap_or(DEFAULT_MAX_HISTORY_SIZE),
            state: Arc::new(Mutex::new(ConversationState {
                steps: Vec::new(),
                turn_start_indices: Vec::new(),
                compaction_indices: Vec::new(),
                cumulative_usage: UsageMetadata::default(),
                turn_usage: None,
            })),
        }
    }

    pub fn connection(&self) -> Arc<dyn Connection> {
        self.conn.clone()
    }

    pub fn conversation_id(&self) -> &str {
        self.conn.conversation_id()
    }

    pub fn is_idle(&self) -> bool {
        self.conn.is_idle()
    }

    pub async fn history(&self) -> Vec<Step> {
        self.state.lock().await.steps.clone()
    }

    pub async fn turn_count(&self) -> usize {
        self.state.lock().await.turn_start_indices.len()
    }

    pub async fn compaction_indices(&self) -> Vec<usize> {
        self.state.lock().await.compaction_indices.clone()
    }

    pub async fn last_response(&self) -> String {
        let state = self.state.lock().await;
        let response = state
            .steps
            .iter()
            .rev()
            .find(|step| step.is_complete_response == Some(true))
            .map(|step| step.content.clone())
            .unwrap_or_default();
        drop(state);
        response
    }

    pub async fn total_usage(&self) -> UsageMetadata {
        self.state.lock().await.cumulative_usage.clone()
    }

    pub async fn last_turn_usage(&self) -> Option<UsageMetadata> {
        self.state.lock().await.turn_usage.clone()
    }

    pub async fn clear_history(&self) {
        let mut state = self.state.lock().await;
        state.steps.clear();
        state.turn_start_indices.clear();
        state.compaction_indices.clear();
        state.cumulative_usage = UsageMetadata::default();
        state.turn_usage = None;
    }

    pub async fn send(&self, prompt: &str) -> Result<(), anyhow::Error> {
        // If not idle, wait for it
        if !self.conn.is_idle() {
            // Note: Unlike Python's runtime RuntimeError handling, in Rust we can just wait
            // or let the stream run-loop handle it.
        }
        let mut state = self.state.lock().await;
        let len = state.steps.len();
        state.turn_start_indices.push(len);
        state.turn_usage = None;
        drop(state);
        self.conn.send(prompt).await
    }

    pub fn receive_steps(&self) -> BoxStream<'static, Result<Step, anyhow::Error>> {
        let conn_stream = self.conn.receive_steps();
        let state = self.state.clone();
        let max_history = self.max_history_size;

        conn_stream
            .then(move |step_res| {
                let state = state.clone();
                async move {
                    match step_res {
                        Ok(step) => {
                            let mut s = state.lock().await;
                            s.steps.push(step.clone());
                            if step.r#type == StepType::Compaction {
                                let len = s.steps.len();
                                s.compaction_indices.push(len - 1);
                            }
                            if let Some(ref usage) = step.usage_metadata {
                                s.cumulative_usage.prompt_token_count += usage.prompt_token_count;
                                s.cumulative_usage.cached_content_token_count +=
                                    usage.cached_content_token_count;
                                s.cumulative_usage.candidates_token_count +=
                                    usage.candidates_token_count;
                                s.cumulative_usage.thoughts_token_count +=
                                    usage.thoughts_token_count;
                                s.cumulative_usage.total_token_count += usage.total_token_count;

                                let mut turn_usage = s.turn_usage.take().unwrap_or_default();
                                turn_usage.prompt_token_count += usage.prompt_token_count;
                                turn_usage.cached_content_token_count +=
                                    usage.cached_content_token_count;
                                turn_usage.candidates_token_count += usage.candidates_token_count;
                                turn_usage.thoughts_token_count += usage.thoughts_token_count;
                                turn_usage.total_token_count += usage.total_token_count;
                                s.turn_usage = Some(turn_usage);
                            }

                            // Enforce max history size
                            if max_history > 0 && s.steps.len() > max_history {
                                let overflow = s.steps.len() - max_history;
                                s.steps.drain(0..overflow);
                                s.turn_start_indices = s
                                    .turn_start_indices
                                    .iter()
                                    .filter_map(|&idx| {
                                        if idx >= overflow {
                                            Some(idx - overflow)
                                        } else {
                                            None
                                        }
                                    })
                                    .collect();
                                s.compaction_indices = s
                                    .compaction_indices
                                    .iter()
                                    .filter_map(|&idx| {
                                        if idx >= overflow {
                                            Some(idx - overflow)
                                        } else {
                                            None
                                        }
                                    })
                                    .collect();
                            }
                            drop(s);

                            Ok(step)
                        }
                        Err(e) => Err(e),
                    }
                }
            })
            .boxed()
    }

    pub fn receive_chunks(&self) -> BoxStream<'static, Result<StreamChunk, anyhow::Error>> {
        let steps = self.receive_steps();
        let mut seen_tool_ids = HashSet::new();

        steps
            .flat_map(move |step_res| {
                let mut chunks = Vec::new();
                match step_res {
                    Ok(step) => {
                        let is_model = step.source == StepSource::Model;
                        let is_target_user = step.target == StepTarget::User;

                        if is_model && is_target_user {
                            if !step.thinking_delta.is_empty() {
                                chunks.push(Ok(StreamChunk::Thought {
                                    step_index: step.step_index,
                                    text: step.thinking_delta.clone(),
                                }));
                            }
                            if !step.content_delta.is_empty() {
                                chunks.push(Ok(StreamChunk::Text {
                                    step_index: step.step_index,
                                    text: step.content_delta.clone(),
                                }));
                            }
                        }

                        for call in step.tool_calls {
                            if call.id.is_empty() || seen_tool_ids.insert(call.id.clone()) {
                                chunks.push(Ok(StreamChunk::ToolCall(call)));
                            }
                        }
                    }
                    Err(e) => {
                        chunks.push(Err(e));
                    }
                }
                stream::iter(chunks)
            })
            .boxed()
    }

    pub async fn chat(
        &self,
        prompt: &str,
    ) -> Result<BoxStream<'static, Result<StreamChunk, anyhow::Error>>, anyhow::Error> {
        self.send(prompt).await?;
        Ok(self.receive_chunks())
    }

    pub async fn chat_to_completion(&self, prompt: &str) -> Result<ChatResponse, anyhow::Error> {
        let mut chunks = self.chat(prompt).await?;
        let mut text = String::new();
        let mut thinking = String::new();
        while let Some(chunk_res) = chunks.next().await {
            match chunk_res? {
                StreamChunk::Text { text: delta, .. } => {
                    text.push_str(&delta);
                }
                StreamChunk::Thought { text: delta, .. } => {
                    thinking.push_str(&delta);
                }
                StreamChunk::ToolCall(_) => {}
            }
        }
        let steps = self.history().await;
        let usage_metadata = self.total_usage().await;
        Ok(ChatResponse {
            text,
            thinking,
            steps,
            usage_metadata,
        })
    }

    pub async fn disconnect(&self) -> Result<(), anyhow::Error> {
        self.conn.disconnect().await
    }
}
