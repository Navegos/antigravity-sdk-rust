use leptos::prelude::*;
use leptos_meta::*;
use leptos_router::{
    components::{Route, Router, Routes},
    *,
};

use crate::types::ChatMessage;

// ── Shell (SSR only) ──────────────────────────────────────────────────────────

#[cfg(feature = "ssr")]
pub fn shell(options: leptos::config::LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <AutoReload options=options.clone() />
                <HydrationScripts options=options.clone() />
                <MetaTags />
            </head>
            <body>
                <App />
            </body>
        </html>
    }
}

// ── App Root ──────────────────────────────────────────────────────────────────

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    let fallback = || view! { "Page not found." }.into_view();

    view! {
        <Stylesheet id="leptos" href="/pkg/leptos_axum_chat.css" />
        <Meta
            name="description"
            content="Antigravity SDK Chat — AI Agent powered by Gemini + antigravity-sdk-rust"
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

// ── Chat Page ─────────────────────────────────────────────────────────────────

#[component]
fn ChatPage() -> impl IntoView {
    // Server action for sending messages
    let send_action = ServerAction::<SendMessage>::new();

    // Chat messages state
    let (messages, set_messages) = signal(Vec::<ChatMessage>::new());
    let (input_text, set_input_text) = signal(String::new());
    let textarea_ref = NodeRef::<leptos::html::Textarea>::new();

    // Load existing messages from server on mount
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
            timestamp: 0,
        };
        set_messages.update(|msgs| {
            msgs.push(user_msg);
        });

        // Clear input
        set_input_text.set(String::new());
        if let Some(el) = textarea_ref.get() {
            el.set_value("");
        }

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
            if let Some(el) = textarea_ref.get() {
                el.set_value("");
            }
            send_action.dispatch(SendMessage { message: trimmed });
        }
    };

    view! {
        <div
            class="min-h-screen bg-[#0f1720] text-white flex flex-col"
            style="font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;"
        >
            // Header
            <header class="bg-[#1a2332]/80 backdrop-blur-md border-b border-[#2a3a4c] px-4 py-3 flex items-center gap-3 sticky top-0 z-50 shrink-0">
                <div class="w-8 h-8 bg-gradient-to-br from-[#00d4aa] to-[#00b4d8] rounded-lg flex items-center justify-center text-[#0f1720] font-bold text-sm">
                    "A"
                </div>
                <div>
                    <h1 class="text-sm font-semibold text-white tracking-tight">
                        "Antigravity Chat"
                    </h1>
                    <p class="text-[#6b7d95] text-xs">
                        "Powered by antigravity-sdk-rust + Gemini"
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
                            <div class="w-4 h-4 bg-[#00d4aa] rounded-full animate-pulse"></div>
                        </div>
                        <h2 class="text-lg font-semibold text-white mb-2">
                            "Start a conversation"
                        </h2>
                        <p class="text-[#6b7d95] text-sm max-w-md">
                            "Ask me anything. I'm an AI agent powered by the Antigravity SDK,"
                            <br />
                            "using the full Agent API with localharness."
                        </p>
                    </div>
                </Show>

                {move || messages.get().into_iter().enumerate().map(|(_i, msg)| {
                    let is_user = msg.role == "user";
                    view! {
                        <div class={if is_user {
                            "flex justify-end"
                        } else {
                            "flex justify-start"
                        }}>
                            <div class={if is_user {
                                "bg-[#00d4aa] text-[#0f1720] rounded-2xl rounded-tr-md px-4 py-2.5 max-w-[75%] text-sm font-medium"
                            } else {
                                "bg-[#1a2332] text-[#e0e6ed] rounded-2xl rounded-tl-md px-4 py-2.5 max-w-[75%] text-sm border border-[#2a3a4c]"
                            }}>
                                <p class="whitespace-pre-wrap">{msg.content.clone()}</p>
                            </div>
                        </div>
                    }
                }).collect::<Vec<_>>()}

                // Loading indicator
                <Show when=move || send_action.pending().get()>
                    <div class="flex justify-start">
                        <div class="bg-[#1a2332] rounded-2xl rounded-tl-md px-4 py-3 border border-[#2a3a4c]">
                            <div class="flex gap-1.5">
                                <div class="w-2 h-2 bg-[#00d4aa] rounded-full animate-bounce" style="animation-delay: 0ms;"></div>
                                <div class="w-2 h-2 bg-[#00d4aa] rounded-full animate-bounce" style="animation-delay: 150ms;"></div>
                                <div class="w-2 h-2 bg-[#00d4aa] rounded-full animate-bounce" style="animation-delay: 300ms;"></div>
                            </div>
                        </div>
                    </div>
                </Show>
            </div>

            // Input area
            <div class="bg-[#1a2332] border-t border-[#2a3a4c] px-4 py-4 shrink-0">
                <div class="max-w-3xl mx-auto flex gap-3">
                    <textarea
                        node_ref=textarea_ref
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

#[component]
fn NotFound() -> impl IntoView {
    view! {
        <div class="min-h-screen bg-[#0f1720] flex items-center justify-center">
            <div class="text-center">
                <h1 class="text-6xl font-bold text-[#00d4aa] mb-4">"404"</h1>
                <p class="text-[#6b7d95] text-lg">"Page not found"</p>
                <a href="/" class="text-[#00d4aa] hover:underline mt-4 inline-block">"Go back home"</a>
            </div>
        </div>
    }
}

// ── Server Functions ────────────────────────────────────────────────────────

/// Send a message to the AI agent using the antigravity-sdk-rust Agent.
#[server(prefix = "/api")]
pub async fn send_message(message: String) -> Result<ChatMessage, ServerFnError<String>> {
    use crate::{AgentState, ChatHistoryState};

    let agent_state =
        use_context::<AgentState>().ok_or_else(|| "Agent not available".to_string())?;
    let chat_history_state =
        use_context::<ChatHistoryState>().ok_or_else(|| "Chat history not available".to_string())?;

    // Record the user message in history
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let user_msg = ChatMessage {
        role: "user".to_string(),
        content: message.clone(),
        timestamp: now,
    };

    {
        let mut history = chat_history_state.lock().await;
        history.push(user_msg);
    }

    // Call the Agent using the SDK!
    let response = {
        let agent = agent_state.lock().await;
        agent
            .chat(&message)
            .await
            .map_err(|e| format!("Agent error: {e}"))?
    };

    // Build assistant message
    let assistant_msg = ChatMessage {
        role: "assistant".to_string(),
        content: response.text,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    };

    // Save to history
    {
        let mut history = chat_history_state.lock().await;
        history.push(assistant_msg.clone());
    }

    Ok(assistant_msg)
}

/// Retrieve chat messages from the in-memory history.
#[server(prefix = "/api")]
pub async fn get_messages() -> Result<Vec<ChatMessage>, ServerFnError<String>> {
    use crate::ChatHistoryState;

    let chat_history_state =
        use_context::<ChatHistoryState>().ok_or_else(|| "Chat history not available".to_string())?;

    let history = chat_history_state.lock().await;
    Ok(history.clone())
}
