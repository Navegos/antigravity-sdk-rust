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

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::field_reassign_with_default,
        clippy::similar_names,
        clippy::single_match,
        clippy::match_wildcard_for_single_variants,
        clippy::manual_string_new
    )]
    use super::*;
    use crate::types::{StepSource, StepTarget, StepType, ToolCall, QuestionHookResult};
    use async_trait::async_trait;
    use futures_util::StreamExt;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Mutex;

    struct MockConnection {
        id: String,
        is_idle: AtomicBool,
        steps_to_yield: Mutex<Vec<Step>>,
        sent_prompts: Mutex<Vec<String>>,
    }

    impl MockConnection {
        fn new(id: &str) -> Self {
            Self {
                id: id.to_string(),
                is_idle: AtomicBool::new(true),
                steps_to_yield: Mutex::new(Vec::new()),
                sent_prompts: Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait]
    impl Connection for MockConnection {
        fn conversation_id(&self) -> &str {
            &self.id
        }

        fn is_idle(&self) -> bool {
            self.is_idle.load(Ordering::SeqCst)
        }

        fn receive_steps(&self) -> BoxStream<'static, Result<Step, anyhow::Error>> {
            let steps = self.steps_to_yield.lock().unwrap().clone();
            futures_util::stream::iter(steps).map(Ok).boxed()
        }

        async fn send(&self, content: &str) -> Result<(), anyhow::Error> {
            self.sent_prompts.lock().unwrap().push(content.to_string());
            Ok(())
        }

        async fn send_trigger_notification(&self, _content: &str) -> Result<(), anyhow::Error> {
            Ok(())
        }

        async fn send_halt_request(&self) -> Result<(), anyhow::Error> {
            Ok(())
        }

        async fn send_tool_confirmation(
            &self,
            _trajectory_id: &str,
            _step_index: u32,
            _accepted: bool,
        ) -> Result<(), anyhow::Error> {
            Ok(())
        }

        async fn send_tool_response(&self, _id: &str, _result: crate::types::ToolResult) -> Result<(), anyhow::Error> {
            Ok(())
        }

        async fn send_question_response(
            &self,
            _trajectory_id: &str,
            _step_index: u32,
            _answers: QuestionHookResult,
        ) -> Result<(), anyhow::Error> {
            Ok(())
        }

        async fn disconnect(&self) -> Result<(), anyhow::Error> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_conversation_initialization() {
        let conn = Arc::new(MockConnection::new("conv-123"));
        let conv = Conversation::new(conn, Some(10));
        assert_eq!(conv.conversation_id(), "conv-123");
        assert!(conv.is_idle());
        assert_eq!(conv.history().await.len(), 0);
        assert_eq!(conv.turn_count().await, 0);
    }

    #[tokio::test]
    async fn test_send_records_turn_boundary() {
        let conn = Arc::new(MockConnection::new("conv-123"));
        let conv = Conversation::new(conn, Some(10));
        conv.send("hello").await.unwrap();
        assert_eq!(conv.turn_count().await, 1);
        conv.send("world").await.unwrap();
        assert_eq!(conv.turn_count().await, 2);
    }

    #[tokio::test]
    async fn test_receive_steps_accumulates_history() {
        let conn = Arc::new(MockConnection::new("conv-123"));
        let step1 = Step {
            id: "1".to_string(),
            step_index: 1,
            r#type: StepType::TextResponse,
            source: StepSource::Model,
            target: StepTarget::User,
            content: "hello".to_string(),
            is_complete_response: Some(true),
            ..Default::default()
        };
        conn.steps_to_yield.lock().unwrap().push(step1);

        let conv = Conversation::new(conn, Some(10));
        let mut steps = conv.receive_steps();
        while let Some(res) = steps.next().await {
            res.unwrap();
        }

        let hist = conv.history().await;
        assert_eq!(hist.len(), 1);
        assert_eq!(hist[0].content, "hello");
        assert_eq!(conv.last_response().await, "hello");
    }

    #[tokio::test]
    async fn test_compaction_indices_tracked() {
        let conn = Arc::new(MockConnection::new("conv-123"));
        let step1 = Step {
            id: "1".to_string(),
            step_index: 1,
            r#type: StepType::Compaction,
            source: StepSource::Model,
            target: StepTarget::User,
            ..Default::default()
        };
        conn.steps_to_yield.lock().unwrap().push(step1);

        let conv = Conversation::new(conn, Some(10));
        let mut steps = conv.receive_steps();
        while let Some(res) = steps.next().await {
            res.unwrap();
        }

        assert_eq!(conv.compaction_indices().await, vec![0]);
    }

    #[tokio::test]
    async fn test_max_history_size_trimming() {
        let conn = Arc::new(MockConnection::new("conv-123"));
        for i in 0..5 {
            conn.steps_to_yield.lock().unwrap().push(Step {
                id: i.to_string(),
                step_index: i,
                content: format!("step-{}", i),
                ..Default::default()
            });
        }

        let conv = Conversation::new(conn.clone(), Some(3));
        let mut steps = conv.receive_steps();
        while let Some(res) = steps.next().await {
            res.unwrap();
        }

        let hist = conv.history().await;
        assert_eq!(hist.len(), 3);
        assert_eq!(hist[0].content, "step-2");
        assert_eq!(hist[2].content, "step-4");
    }

    #[tokio::test]
    async fn test_max_history_size_zero_disables_trimming() {
        let conn = Arc::new(MockConnection::new("conv-123"));
        for i in 0..5 {
            conn.steps_to_yield.lock().unwrap().push(Step {
                id: i.to_string(),
                step_index: i,
                content: format!("step-{}", i),
                ..Default::default()
            });
        }

        let conv = Conversation::new(conn.clone(), Some(0));
        let mut steps = conv.receive_steps();
        while let Some(res) = steps.next().await {
            res.unwrap();
        }

        let hist = conv.history().await;
        assert_eq!(hist.len(), 5);
    }

    #[tokio::test]
    async fn test_receive_chunks_routing() {
        let conn = Arc::new(MockConnection::new("conv-123"));
        let step = Step {
            id: "1".to_string(),
            step_index: 1,
            r#type: StepType::TextResponse,
            source: StepSource::Model,
            target: StepTarget::User,
            content_delta: "hello".to_string(),
            thinking_delta: "reasoning".to_string(),
            ..Default::default()
        };
        conn.steps_to_yield.lock().unwrap().push(step);

        let conv = Conversation::new(conn, Some(10));
        let mut chunks = conv.receive_chunks();
        let mut text = String::new();
        let mut thought = String::new();
        while let Some(res) = chunks.next().await {
            match res.unwrap() {
                StreamChunk::Text { text: delta, .. } => text.push_str(&delta),
                StreamChunk::Thought { text: delta, .. } => thought.push_str(&delta),
                _ => {}
            }
        }

        assert_eq!(text, "hello");
        assert_eq!(thought, "reasoning");
    }

    #[tokio::test]
    async fn test_receive_chunks_environmental_filtering() {
        let conn = Arc::new(MockConnection::new("conv-123"));
        let step = Step {
            id: "1".to_string(),
            step_index: 1,
            r#type: StepType::TextResponse,
            source: StepSource::Model,
            target: StepTarget::Environment,
            content_delta: "env content".to_string(),
            ..Default::default()
        };
        conn.steps_to_yield.lock().unwrap().push(step);

        let conv = Conversation::new(conn, Some(10));
        let mut chunks = conv.receive_chunks();
        let mut text = String::new();
        while let Some(res) = chunks.next().await {
            match res.unwrap() {
                StreamChunk::Text { text: delta, .. } => text.push_str(&delta),
                _ => {}
            }
        }

        assert_eq!(text, "");
    }

    #[tokio::test]
    async fn test_receive_chunks_tool_calls_deduplication() {
        let conn = Arc::new(MockConnection::new("conv-123"));
        let tc = ToolCall {
            id: "call_a".to_string(),
            name: "tool_1".to_string(),
            args: serde_json::Value::Null,
            canonical_path: None,
        };
        let step = Step {
            id: "1".to_string(),
            step_index: 1,
            r#type: StepType::ToolCall,
            source: StepSource::Model,
            target: StepTarget::User,
            tool_calls: vec![tc.clone(), tc.clone()],
            ..Default::default()
        };
        conn.steps_to_yield.lock().unwrap().push(step);

        let conv = Conversation::new(conn, Some(10));
        let mut chunks = conv.receive_chunks();
        let mut tool_calls = Vec::new();
        while let Some(res) = chunks.next().await {
            if let StreamChunk::ToolCall(c) = res.unwrap() {
                tool_calls.push(c);
            }
        }

        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].id, "call_a");
    }

    #[tokio::test]
    async fn test_receive_chunks_empty_tool_id_no_dedup() {
        let conn = Arc::new(MockConnection::new("conv-123"));
        let tc = ToolCall {
            id: "".to_string(),
            name: "tool_1".to_string(),
            args: serde_json::Value::Null,
            canonical_path: None,
        };
        let step = Step {
            id: "1".to_string(),
            step_index: 1,
            r#type: StepType::ToolCall,
            source: StepSource::Model,
            target: StepTarget::User,
            tool_calls: vec![tc.clone(), tc.clone()],
            ..Default::default()
        };
        conn.steps_to_yield.lock().unwrap().push(step);

        let conv = Conversation::new(conn, Some(10));
        let mut chunks = conv.receive_chunks();
        let mut tool_calls = Vec::new();
        while let Some(res) = chunks.next().await {
            if let StreamChunk::ToolCall(c) = res.unwrap() {
                tool_calls.push(c);
            }
        }

        assert_eq!(tool_calls.len(), 2);
    }
}

