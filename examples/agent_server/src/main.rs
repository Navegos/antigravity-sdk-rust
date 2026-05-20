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

use antigravity_sdk_rust::agent::{Agent, AgentConfig};
use antigravity_sdk_rust::policy;
use antigravity_sdk_rust::types::{CustomSystemInstructions, GeminiConfig, SystemInstructions};
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::EnvFilter;

/// Shared application state containing the SDK Agent.
type AgentState = Arc<Mutex<Agent>>;

/// Request body for the `/chat` endpoint.
#[derive(Debug, Deserialize)]
struct ChatRequest {
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
    let agent_guard = agent.lock().await;

    match agent_guard.chat(&req.message).await {
        Ok(response) => {
            let conversation_id = agent_guard.conversation_id();
            drop(agent_guard);

            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "text": response.text,
                    "conversation_id": conversation_id,
                })),
            )
        }
        Err(e) => {
            drop(agent_guard);
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
    let mut agent_config = AgentConfig::default();

    // Configure Gemini
    let mut gemini_config = GeminiConfig::default();
    if let Ok(api_key) = std::env::var("GEMINI_API_KEY") {
        gemini_config.api_key = Some(api_key);
    }
    gemini_config.models.default.name = std::env::var("GEMINI_MODEL")
        .unwrap_or_else(|_| "gemini-3.5-flash".to_string());
    agent_config.gemini_config = gemini_config;

    // Set policies — allow all for this demo
    agent_config.policies = Some(vec![policy::allow_all()]);

    // Set custom system instructions
    agent_config.system_instructions = Some(SystemInstructions::Custom(
        CustomSystemInstructions {
            text: "You are a helpful AI assistant in a chat interface. \
                   Keep responses concise and conversational. \
                   Use markdown formatting when helpful."
                .to_string(),
        },
    ));

    // Check for harness binary path
    if let Ok(harness_path) = std::env::var("ANTIGRAVITY_HARNESS_PATH") {
        agent_config.binary_path = Some(harness_path);
    }

    // Disable all file/command tools — this is a chat-only sidecar
    agent_config.capabilities.enabled_tools = Some(vec![]);

    // Start the Agent
    let mut agent = Agent::new(agent_config);
    tracing::info!("Starting antigravity-sdk-rust Agent...");
    agent
        .start()
        .await
        .expect("Failed to start Agent. Is localharness installed? Set ANTIGRAVITY_HARNESS_PATH if needed.");
    tracing::info!("Agent started successfully");

    let agent_state: AgentState = Arc::new(Mutex::new(agent));

    // 5. Build Axum router
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/chat", post(chat_handler))
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
