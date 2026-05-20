#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use antigravity_sdk_rust::agent::Agent;
    use axum::Router;
    use leptos::prelude::*;
    use leptos_axum::{generate_route_list, LeptosRoutes};
    use leptos_axum_chat::app::{shell, App};
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use tracing_subscriber::EnvFilter;

    // 1. Load .env for GEMINI_API_KEY
    dotenvy::dotenv().ok();

    // 2. Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    // 3. Get Leptos configuration
    let conf = get_configuration(None).expect("Failed to get Leptos configuration");
    let leptos_options = conf.leptos_options.clone();
    let addr = leptos_options.site_addr;

    // 4. Initialize the antigravity-sdk-rust Agent
    let harness_path = std::env::var("ANTIGRAVITY_HARNESS_PATH").ok();
    let api_key = std::env::var("GEMINI_API_KEY").ok();

    let mut builder = Agent::builder();
    if let Some(path) = harness_path {
        builder = builder.binary_path(path);
    }
    if let Some(key) = api_key {
        builder = builder.api_key(key);
    }

    let agent = builder
        .default_model("gemini-3.5-flash")
        .allow_all()
        .system_instructions(antigravity_sdk_rust::types::SystemInstructions::Custom(
            antigravity_sdk_rust::types::CustomSystemInstructions {
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

    tracing::info!("Starting antigravity-sdk-rust Agent...");
    let agent = agent
        .start()
        .await
        .expect("Failed to start Agent. Is localharness installed?");
    tracing::info!("Agent started successfully");

    // Wrap agent in shared state
    let agent_state: leptos_axum_chat::AgentState = Arc::new(agent);

    // Also keep chat history in memory for this session
    let chat_history: leptos_axum_chat::ChatHistoryState =
        Arc::new(Mutex::new(Vec::new()));

    // 5. Build routes
    let routes = generate_route_list(App);

    // 6. Build Axum router
    // Serve static files from target/site (JS/WASM/CSS) before Leptos SSR routes
    let site_root = leptos_options.site_root.clone();
    let app = Router::new()
        .nest_service(
            "/pkg",
            tower_http::services::ServeDir::new(format!("{}/pkg", &site_root)),
        )
        .leptos_routes_with_context(
            &leptos_options,
            routes,
            {
                let agent_state = agent_state.clone();
                let chat_history = chat_history.clone();
                move || {
                    leptos::context::provide_context(agent_state.clone());
                    leptos::context::provide_context(chat_history.clone());
                }
            },
            {
                let leptos_options = leptos_options.clone();
                move || shell(leptos_options.clone())
            },
        )
        .fallback(leptos_axum::file_and_error_handler(shell))
        .with_state(leptos_options);

    // 7. Serve
    tracing::info!("Listening on http://{}", &addr);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind");
    axum::serve(listener, app.into_make_service())
        .await
        .expect("Server error");
}

#[cfg(not(feature = "ssr"))]
pub fn main() {
    // no-op: client-side only
}
