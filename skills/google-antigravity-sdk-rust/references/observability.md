# Observability & Token Metrics

This guide explains how to track token usage, check cost configurations, and analyze reasoning model thoughts.

## Usage Metadata

Each conversation `Step` returned by the WebSocket server contains a `usage_metadata` field (mapped to the `UsageMetadata` struct):

```rust
pub struct UsageMetadata {
    pub prompt_token_count: u32,
    pub candidates_token_count: u32,
    pub total_token_count: u32,
    pub cached_content_token_count: u32,
    pub thoughts_token_count: u32,
}
```

* **`prompt_token_count`**: Number of tokens in the input prompt.
* **`candidates_token_count`**: Number of tokens in the generated response (excluding thinking).
* **`thoughts_token_count`**: Number of tokens consumed by the reasoning model's internal thinking process.
* **`total_token_count`**: Total tokens consumed by the step.
* **`cached_content_token_count`**: Tokens loaded from prompt caching.

---

## Accessing Metrics from ChatResponse

When using `Agent::chat()` or `Conversation::chat_to_completion()`, you receive a `ChatResponse` containing a list of `Step` structures. You can iterate through these to print usage statistics:

```rust
let response = agent.chat("Plan a refactor of our tests").await?;

for (idx, step) in response.steps.iter().enumerate() {
    if let Some(ref usage) = step.usage_metadata {
        println!("Step {}: Total tokens used: {}", idx, usage.total_token_count);
        println!("  - Prompt:   {}", usage.prompt_token_count);
        println!("  - Response: {}", usage.candidates_token_count);
        println!("  - Thinking: {}", usage.thoughts_token_count);
        println!("  - Cached:   {}", usage.cached_content_token_count);
    }
}
```

---

## Observing Real-Time Thoughts (Streaming)

When consuming the conversation stream using the asynchronous streaming API, chunks are delivered as the `StreamChunk` enum. This allows you to differentiate between thoughts and text responses:

```rust
use antigravity_sdk_rust::types::StreamChunk;
use futures_util::StreamExt;

let conversation = agent.conversation()?;
let mut stream = conversation.chat("How does rust memory safety work?").await?;

while let Some(chunk_res) = stream.next().await {
    match chunk_res? {
        StreamChunk::Thought { text, .. } => {
            // Render reasoning thoughts (e.g. print in yellow or brackets)
            print!("[Thinking: {}]", text);
        }
        StreamChunk::Text { text, .. } => {
            // Render standard response output
            print!("{}", text);
        }
        StreamChunk::ToolCall(call) => {
            println!("\n[Tool Call Invoked: {}]", call.name);
        }
    }
    std::io::Write::flush(&mut std::io::stdout())?;
}
```
