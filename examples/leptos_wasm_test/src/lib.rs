use antigravity_sdk_rust::connection::Connection;
use antigravity_sdk_rust::types::{QuestionHookResult, Step, ToolResult};
use async_trait::async_trait;
use futures_util::stream::{self, BoxStream};
use leptos::prelude::*;

#[allow(dead_code)]
struct BrowserMockConnection;

#[async_trait]
impl Connection for BrowserMockConnection {
    fn conversation_id(&self) -> &str {
        "wasm_conv"
    }

    fn is_idle(&self) -> bool {
        true
    }

    fn receive_steps(&self) -> BoxStream<'static, Result<Step, anyhow::Error>> {
        Box::pin(stream::empty())
    }

    async fn send(&self, _content: &str) -> Result<(), anyhow::Error> {
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

    async fn send_tool_response(&self, _id: &str, _result: ToolResult) -> Result<(), anyhow::Error> {
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

#[component]
pub fn App() -> impl IntoView {
    let (status, set_status) = signal("Initializing...".to_string());

    let run_agent = move |_| {
        set_status.set("Running browser mock connection...".to_string());
    };

    view! {
        <div>
            <h1>"Welcome to Leptos WASM Antigravity Test"</h1>
            <p>"Status: " {move || status.get()}</p>
            <button on:click=run_agent>"Run Agent"</button>
        </div>
    }
}

#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(App);
}
