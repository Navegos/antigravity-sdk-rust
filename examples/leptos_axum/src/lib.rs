#![recursion_limit = "512"]

pub mod app;
pub mod types;

#[cfg(feature = "ssr")]
use antigravity_sdk_rust::agent::{Agent, Started};
#[cfg(feature = "ssr")]
use std::sync::Arc;
#[cfg(feature = "ssr")]
use tokio::sync::Mutex;

/// Shared Agent state accessible from server functions via Leptos context.
#[cfg(feature = "ssr")]
pub type AgentState = Arc<Agent<Started>>;

/// In-memory chat history for the current session.
#[cfg(feature = "ssr")]
pub type ChatHistoryState = Arc<Mutex<Vec<types::ChatMessage>>>;

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    use app::App;

    console_error_panic_hook::set_once();

    leptos::mount::hydrate_body(App);
}
