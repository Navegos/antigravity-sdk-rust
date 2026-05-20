//! HTTP Sidecar Server for the Antigravity SDK.
//!
//! This binary wraps the full `antigravity-sdk-rust` Agent (including localharness,
//! tools, hooks, and policies) and exposes it via a simple HTTP REST API.
//!
//! Spin/WASI components cannot make outbound TCP/WebSocket connections — only `wasi::http`.
//! This sidecar bridges that gap: it runs natively, maintains the WebSocket connection
//! to localharness, and exposes `POST /chat` for the Spin component to call.
//!
//! # Usage
//!
//! ```bash
//! # Terminal 1: Start the sidecar
//! cd examples/agent_server
//! GEMINI_API_KEY=your-key cargo run
//!
//! # Terminal 2: Start Spin
//! cd examples/leptos_ssr_axum
//! spin build --up
//! ```

use antigravity_sdk_rust::agent::{Agent, Started};
use antigravity_sdk_rust::types::{CustomSystemInstructions, SystemInstructions};
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::EnvFilter;
use axum::response::sse::{Event, KeepAlive, Sse};
use futures_util::StreamExt;
use std::convert::Infallible;

/// Shared application state containing the SDK Agent.
type AgentState = Arc<Agent<Started>>;

/// Request body for the `/chat` endpoint.
#[derive(Debug, Deserialize)]
struct ChatRequest {
    message: String,
}

/// Query parameters for the `/chat/stream` endpoint.
#[derive(Debug, Deserialize)]
struct ChatStreamParams {
    message: String,
}

/// POST /chat — Send a message to the Agent and get a response.
///
/// This calls `agent.chat(&message)` which goes through the full SDK pipeline:
/// localharness → Gemini API → tool execution → response.
async fn chat_handler(
    State(agent): State<AgentState>,
    Json(req): Json<ChatRequest>,
) -> impl IntoResponse {
    match agent.chat(&req.message).await {
        Ok(response) => {
            let conversation_id = agent.conversation_id();

            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "text": response.text,
                    "conversation_id": conversation_id,
                })),
            )
        }
        Err(e) => {
            tracing::error!("Agent chat error: {:?}", e);

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": format!("{e}"),
                })),
            )
        }
    }
}

/// GET /chat/stream — Send a message and stream the response back via Server-Sent Events (SSE).
async fn chat_stream_handler(
    State(agent): State<AgentState>,
    axum::extract::Query(params): axum::extract::Query<ChatStreamParams>,
) -> impl IntoResponse {
    let conversation = agent.conversation();

    let stream_res = conversation.chat(&params.message).await;

    let sse_stream: futures_util::stream::BoxStream<'static, Result<Event, Infallible>> = match stream_res {
        Ok(stream) => {
            let sse = stream.map(|chunk_res| {
                match chunk_res {
                    Ok(chunk) => {
                        match chunk {
                            antigravity_sdk_rust::types::StreamChunk::Text { text, .. } => {
                                Ok(Event::default().event("token").json_data(serde_json::json!({ "text": text })).unwrap())
                            }
                            antigravity_sdk_rust::types::StreamChunk::Thought { text, .. } => {
                                Ok(Event::default().event("thought").json_data(serde_json::json!({ "text": text })).unwrap())
                            }
                            antigravity_sdk_rust::types::StreamChunk::ToolCall(call) => {
                                Ok(Event::default().event("tool").json_data(serde_json::json!({
                                    "name": call.name,
                                    "args": call.args,
                                })).unwrap())
                            }
                        }
                    }
                    Err(e) => {
                        Ok(Event::default().event("error").data(format!("{e}")))
                    }
                }
            });

            let done_event = futures_util::stream::once(async {
                Ok(Event::default()
                    .event("done")
                    .json_data(serde_json::json!({}))
                    .unwrap())
            });

            sse.chain(done_event).boxed()
        }
        Err(e) => {
            let error_event = futures_util::stream::once(async move {
                Ok(Event::default().event("error").data(format!("{e}")))
            });
            error_event.boxed()
        }
    };

    Sse::new(sse_stream).keep_alive(KeepAlive::default())
}

/// GET /health — Health check endpoint.
async fn health_handler() -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({"status": "ok"})))
}

#[tokio::main]
async fn main() {
    // 1. Load .env for GEMINI_API_KEY
    dotenvy::dotenv().ok();

    // 2. Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    // 3. Read config from environment
    let port: u16 = std::env::var("AGENT_SERVER_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);

    // 4. Initialize the antigravity-sdk-rust Agent
    let harness_path = std::env::var("ANTIGRAVITY_HARNESS_PATH").ok();
    let api_key = std::env::var("GEMINI_API_KEY").ok();
    let model = std::env::var("GEMINI_MODEL")
        .unwrap_or_else(|_| "gemini-3.5-flash".to_string());

    let mut builder = Agent::builder();
    if let Some(path) = harness_path {
        builder = builder.binary_path(path);
    }
    if let Some(key) = api_key {
        builder = builder.api_key(key);
    }

    let agent = builder
        .default_model(model)
        .allow_all()
        .system_instructions(SystemInstructions::Custom(
            CustomSystemInstructions {
                text: "You are a helpful AI assistant in a chat interface. \
                       Keep responses concise and conversational. \
                       Use markdown formatting when helpful."
                    .to_string(),
            },
        ))
        .capabilities(antigravity_sdk_rust::types::CapabilitiesConfig {
            enabled_tools: Some(vec![]),
            disabled_tools: None,
            compaction_threshold: None,
            image_model: None,
            finish_tool_schema_json: None,
        })
        .build();

    // Start the Agent
    tracing::info!("Starting antigravity-sdk-rust Agent...");
    let agent = agent
        .start()
        .await
        .expect("Failed to start Agent. Is localharness installed? Set ANTIGRAVITY_HARNESS_PATH if needed.");
    tracing::info!("Agent started successfully");

    let agent_state: AgentState = Arc::new(agent);

    // 5. Build Axum router
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/chat", post(chat_handler))
        .route("/chat/stream", get(chat_stream_handler))
        .route("/health", get(health_handler))
        .layer(cors)
        .with_state(agent_state);

    // 6. Serve
    let addr = format!("127.0.0.1:{port}");
    tracing::info!("Agent sidecar listening on http://{}", &addr);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind");
    axum::serve(listener, app.into_make_service())
        .await
        .expect("Server error");
}
