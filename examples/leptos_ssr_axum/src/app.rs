use components::{Route, Router, Routes};
use leptos::prelude::*;
use leptos_meta::*;
use leptos_router::*;

use crate::types::ChatMessage;

#[cfg(feature = "ssr")]
pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <AutoReload options=options.clone() />
                <HydrationScripts options=options.clone() root="" />
                <MetaTags />
            </head>
            <body>
                <App />
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    let fallback = || view! { "Page not found." }.into_view();

    view! {
        <Stylesheet id="leptos" href="/pkg/leptos_ssr_axum.css" />
        <Meta
            name="description"
            content="Antigravity SDK Chat — AI Agent powered by Gemini, running on Spin WASI"
        />

        <Title text="Antigravity Chat" />

        <Router>
            <main>
                <Routes fallback>
                    <Route path=path!("") view=ChatPage />
                    <Route path=path!("/*any") view=NotFound />
                </Routes>
            </main>
        </Router>
    }
}

/// Chat page component
#[component]
fn ChatPage() -> impl IntoView {
    let send_action = ServerAction::<SendMessage>::new();

    // Chat messages state
    let (messages, set_messages) = signal(Vec::<ChatMessage>::new());
    let (input_text, set_input_text) = signal(String::new());

    // Load existing messages from KV store on mount
    let initial_messages = Resource::new(|| (), |_| get_messages());

    Effect::new(move |_| {
        if let Some(Ok(msgs)) = initial_messages.get() {
            if !msgs.is_empty() {
                set_messages.set(msgs);
            }
        }
    });

    // When server action completes, append the assistant response
    Effect::new(move |_| {
        if let Some(Ok(assistant_msg)) = send_action.value().get() {
            set_messages.update(|msgs| {
                msgs.push(assistant_msg);
            });
        }
    });

    // Send handler
    let on_send = move |_| {
        let text = input_text.get();
        let trimmed = text.trim().to_string();
        if trimmed.is_empty() {
            return;
        }

        // Optimistically add user message to UI
        let user_msg = ChatMessage {
            role: "user".to_string(),
            content: trimmed.clone(),
            timestamp: 0, // Will be set server-side
        };
        set_messages.update(|msgs| {
            msgs.push(user_msg);
        });

        // Clear input
        set_input_text.set(String::new());

        // Dispatch to server
        send_action.dispatch(SendMessage { message: trimmed });
    };

    // Handle Enter key
    let on_keydown = move |ev: leptos::ev::KeyboardEvent| {
        if ev.key() == "Enter" && !ev.shift_key() {
            ev.prevent_default();
            let text = input_text.get();
            let trimmed = text.trim().to_string();
            if trimmed.is_empty() {
                return;
            }

            let user_msg = ChatMessage {
                role: "user".to_string(),
                content: trimmed.clone(),
                timestamp: 0,
            };
            set_messages.update(|msgs| {
                msgs.push(user_msg);
            });
            set_input_text.set(String::new());
            send_action.dispatch(SendMessage { message: trimmed });
        }
    };

    view! {
        <div class="min-h-screen bg-[#0f1720] flex flex-col">
            // Header
            <header class="bg-[#1a2332] border-b border-[#2a3a4c] px-6 py-4 flex items-center gap-3 shrink-0">
                <div class="w-9 h-9 bg-[#00d4aa] rounded-lg flex items-center justify-center">
                    <span class="text-[#0f1720] font-bold text-lg">A</span>
                </div>
                <div>
                    <h1 class="text-white text-lg font-medium leading-tight">
                        "Antigravity Chat"
                    </h1>
                    <p class="text-[#6b7d95] text-xs">
                        "Powered by Gemini + Spin WASI"
                    </p>
                </div>
                <div class="ml-auto flex items-center gap-2">
                    <div class={move || {
                        if send_action.pending().get() {
                            "w-2 h-2 rounded-full bg-[#00d4aa] animate-pulse"
                        } else {
                            "w-2 h-2 rounded-full bg-[#00d4aa]"
                        }
                    }}></div>
                    <span class="text-[#6b7d95] text-xs uppercase tracking-wider">
                        {move || {
                            if send_action.pending().get() {
                                "Thinking..."
                            } else {
                                "Ready"
                            }
                        }}
                    </span>
                </div>
            </header>

            // Messages area
            <div class="flex-1 overflow-y-auto px-4 py-6 space-y-4" id="chat-messages">
                // Empty state
                <Show when=move || messages.get().is_empty() && !send_action.pending().get()>
                    <div class="flex flex-col items-center justify-center h-full text-center py-20">
                        <div class="w-16 h-16 bg-[#1a2332] rounded-2xl flex items-center justify-center mb-6 border border-[#2a3a4c]">
                            <span class="text-[#00d4aa] text-2xl">"*"</span>
                        </div>
                        <h2 class="text-white text-xl font-medium mb-2">
                            "Start a conversation"
                        </h2>
                        <p class="text-[#6b7d95] text-sm max-w-md">
                            "Ask me anything. I'm an AI agent powered by the Antigravity SDK, "
                            "running server-side as a WASI component on Fermyon Spin."
                        </p>
                    </div>
                </Show>

                // Message bubbles
                <For
                    each=move || messages.get()
                    key=|msg| format!("{}-{}", msg.role, msg.timestamp)
                    let:msg
                >
                    {
                        let is_user = msg.role == "user";
                        let content = msg.content.clone();
                        view! {
                            <div class={if is_user {
                                "flex justify-end"
                            } else {
                                "flex justify-start"
                            }}>
                                <div class={if is_user {
                                    "max-w-[75%] bg-[#00d4aa] text-[#0f1720] rounded-2xl rounded-br-md px-4 py-3 text-sm leading-relaxed"
                                } else {
                                    "max-w-[75%] bg-[#1a2332] text-[#e1e8f0] rounded-2xl rounded-bl-md px-4 py-3 text-sm leading-relaxed border border-[#2a3a4c]"
                                }}>
                                    <pre class="whitespace-pre-wrap font-sans m-0">{content}</pre>
                                </div>
                            </div>
                        }
                    }
                </For>

                // Thinking indicator
                <Show when=move || send_action.pending().get()>
                    <div class="flex justify-start">
                        <div class="bg-[#1a2332] text-[#6b7d95] rounded-2xl rounded-bl-md px-4 py-3 text-sm border border-[#2a3a4c] flex items-center gap-2">
                            <div class="flex gap-1">
                                <div class="w-1.5 h-1.5 bg-[#00d4aa] rounded-full animate-bounce" style="animation-delay: 0ms"></div>
                                <div class="w-1.5 h-1.5 bg-[#00d4aa] rounded-full animate-bounce" style="animation-delay: 150ms"></div>
                                <div class="w-1.5 h-1.5 bg-[#00d4aa] rounded-full animate-bounce" style="animation-delay: 300ms"></div>
                            </div>
                            <span>"Agent is thinking..."</span>
                        </div>
                    </div>
                </Show>
            </div>

            // Input area
            <div class="bg-[#1a2332] border-t border-[#2a3a4c] px-4 py-4 shrink-0">
                <div class="max-w-3xl mx-auto flex gap-3">
                    <textarea
                        prop:value=move || input_text.get()
                        on:input=move |ev| set_input_text.set(event_target_value(&ev))
                        on:keydown=on_keydown
                        disabled=move || send_action.pending().get()
                        placeholder="Type a message..."
                        rows="1"
                        class="flex-1 bg-[#0f1720] text-white placeholder-[#4a5a6c] border border-[#2a3a4c] rounded-xl px-4 py-3 text-sm resize-none focus:outline-none focus:border-[#00d4aa] focus:ring-1 focus:ring-[#00d4aa]/30 transition-colors disabled:opacity-50"
                    ></textarea>
                    <button
                        on:click=on_send
                        disabled=move || send_action.pending().get() || input_text.get().trim().is_empty()
                        class="bg-[#00d4aa] text-[#0f1720] font-medium px-5 py-3 rounded-xl transition-all duration-200 hover:bg-[#00b894] active:scale-[0.97] disabled:opacity-40 disabled:cursor-not-allowed disabled:hover:bg-[#00d4aa] text-sm shrink-0"
                    >
                        {move || if send_action.pending().get() {
                            "..."
                        } else {
                            "Send"
                        }}
                    </button>
                </div>
                <p class="text-center text-[#3a4a5c] text-xs mt-2">
                    "Press Enter to send · Shift+Enter for new line"
                </p>
            </div>
        </div>
    }
}

/// 404 - Not Found
#[component]
fn NotFound() -> impl IntoView {
    #[cfg(feature = "ssr")]
    {
        if let Some(resp) = use_context::<leptos_wasi::response::ResponseOptions>() {
            resp.set_status(leptos_wasi::prelude::StatusCode::NOT_FOUND);
        }
    }

    view! { <h1 class="text-white text-center py-20 text-2xl">"Not Found"</h1> }
}

// ── Server Functions ────────────────────────────────────────────────────────

/// Helper: call Gemini API synchronously using wasi:http outgoing handler.
///
/// We can't use `spin_sdk::http::send().await` inside a `#[server]` fn because
/// the future is `!Send` (uses `Rc<RefCell>`), but Leptos requires `Send`.
/// Instead, we build and send the HTTP request using the raw wasi crate types
/// directly and block on the response pollable.
#[cfg(feature = "ssr")]
fn call_gemini(api_key: &str, contents: &serde_json::Value) -> Result<String, String> {
    use wasi::http::types::{Fields, Method, OutgoingBody, OutgoingRequest, Scheme};

    let model = "gemini-2.5-flash";
    let path_and_query = format!(
        "/v1beta/models/{}:generateContent?key={}",
        model, api_key
    );

    let request_body = serde_json::json!({
        "contents": contents,
        "generationConfig": {
            "temperature": 0.7,
            "maxOutputTokens": 2048
        }
    });

    let body_bytes = serde_json::to_vec(&request_body).map_err(|e| e.to_string())?;

    // Build the request using the wasi:http types
    let headers = Fields::from_list(&[
        ("content-type".to_string(), "application/json".into()),
        ("content-length".to_string(), body_bytes.len().to_string().into()),
    ]).map_err(|e| format!("Failed to create headers: {e:?}"))?;

    let outgoing_req = OutgoingRequest::new(headers);
    outgoing_req
        .set_method(&Method::Post)
        .map_err(|_| "Failed to set method".to_string())?;
    outgoing_req
        .set_scheme(Some(&Scheme::Https))
        .map_err(|_| "Failed to set scheme".to_string())?;
    outgoing_req
        .set_authority(Some("generativelanguage.googleapis.com"))
        .map_err(|_| "Failed to set authority".to_string())?;
    outgoing_req
        .set_path_with_query(Some(&path_and_query))
        .map_err(|_| "Failed to set path".to_string())?;

    // Write body
    let out_body = outgoing_req.body().map_err(|_| "Failed to get body handle".to_string())?;
    {
        let stream = out_body.write().map_err(|_| "Failed to get write stream".to_string())?;
        stream.blocking_write_and_flush(&body_bytes)
            .map_err(|e| format!("Failed to write body: {e:?}"))?;
        // Must drop the stream before finishing the body
    }

    // Finish the body (signals end of request)
    OutgoingBody::finish(out_body, None)
        .map_err(|e| format!("Failed to finish body: {e:?}"))?;

    // Send via wasi:http/outgoing-handler
    let future_response = wasi::http::outgoing_handler::handle(outgoing_req, None)
        .map_err(|e| format!("Failed to send request: {e:?}"))?;

    // Block until we get the response
    let incoming_resp = loop {
        if let Some(result) = future_response.get() {
            break result
                .map_err(|_| "Response already consumed".to_string())?
                .map_err(|e| format!("HTTP error: {e:?}"))?;
        }
        future_response.subscribe().block();
    };

    let status = incoming_resp.status();
    let resp_body_handle = incoming_resp.consume().map_err(|_| "Failed to consume body".to_string())?;
    let resp_stream = resp_body_handle.stream().map_err(|_| "Failed to get stream".to_string())?;

    // Read response body
    let mut resp_bytes = Vec::new();
    loop {
        match resp_stream.blocking_read(65536) {
            Ok(chunk) => resp_bytes.extend_from_slice(&chunk),
            Err(wasi::io::streams::StreamError::Closed) => break,
            Err(e) => return Err(format!("Failed to read response: {e:?}")),
        }
    }

    if status != 200 {
        let err_text = String::from_utf8_lossy(&resp_bytes);
        return Err(format!("Gemini API error ({}): {}", status, err_text));
    }

    let resp_json: serde_json::Value =
        serde_json::from_slice(&resp_bytes).map_err(|e| e.to_string())?;

    let text = resp_json["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .unwrap_or("(No response)")
        .to_string();

    Ok(text)
}

/// Send a message to the Gemini API and get a response.
#[server(prefix = "/api")]
pub async fn send_message(message: String) -> Result<ChatMessage, ServerFnError<String>> {
    let api_key = std::env::var("GEMINI_API_KEY")
        .map_err(|_| "GEMINI_API_KEY not set".to_string())?;

    // Load chat history for context
    let store = spin_sdk::key_value::Store::open_default().map_err(|e| e.to_string())?;
    let history: Vec<ChatMessage> = match store.get_json::<Vec<ChatMessage>>("chat_messages") {
        Ok(Some(msgs)) => msgs,
        _ => Vec::new(),
    };

    // Build conversation contents array for Gemini
    let mut contents = Vec::new();
    for msg in &history {
        let role = if msg.role == "user" { "user" } else { "model" };
        contents.push(serde_json::json!({
            "role": role,
            "parts": [{ "text": msg.content }]
        }));
    }
    // Add the new user message
    contents.push(serde_json::json!({
        "role": "user",
        "parts": [{ "text": message }]
    }));

    let contents_value = serde_json::Value::Array(contents);

    // Call Gemini synchronously (bypasses !Send issue)
    let text = call_gemini(&api_key, &contents_value)?;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let assistant_msg = ChatMessage {
        role: "assistant".to_string(),
        content: text,
        timestamp: now,
    };

    // Persist messages to KV store
    let mut messages = history;

    // Add user message
    messages.push(ChatMessage {
        role: "user".to_string(),
        content: message,
        timestamp: now.saturating_sub(1),
    });

    // Add assistant message
    messages.push(assistant_msg.clone());

    // Save back
    store
        .set_json("chat_messages", &messages)
        .map_err(|e| ServerFnError::ServerError(e.to_string()))?;

    Ok(assistant_msg)
}

/// Get all chat messages from the KV store.
#[server(prefix = "/api")]
pub async fn get_messages() -> Result<Vec<ChatMessage>, ServerFnError<String>> {
    let store = spin_sdk::key_value::Store::open_default().map_err(|e| e.to_string())?;
    match store.get_json::<Vec<ChatMessage>>("chat_messages") {
        Ok(Some(msgs)) => Ok(msgs),
        Ok(None) => Ok(Vec::new()),
        Err(e) => {
            eprintln!("Error loading messages: {e}");
            Ok(Vec::new())
        }
    }
}
