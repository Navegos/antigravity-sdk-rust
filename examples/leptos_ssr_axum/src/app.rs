
use components::{Route, Router, Routes};
use leptos::prelude::*;
use leptos_meta::*;
use leptos_router::*;
use leptos::either::Either;

use crate::types::ChatMessage;

#[cfg(feature = "ssr")]
pub fn shell(options: LeptosOptions) -> impl IntoView {
    let agent_url = spin_sdk::variables::get("agent_server_url")
        .unwrap_or_else(|_| "http://127.0.0.1:8080".to_string());
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <meta name="agent-server-url" content=agent_url />
                <link rel="preconnect" href="https://fonts.googleapis.com" />
                <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin="true" />
                <link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&display=swap" rel="stylesheet" />
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
    let clear_action = ServerAction::<ClearMessages>::new();

    // Chat messages state
    let (messages, set_messages) = signal(Vec::<ChatMessage>::new());
    let (input_text, set_input_text) = signal(String::new());
    let (error_text, set_error_text) = signal(Option::<String>::None);
    let textarea_ref = NodeRef::<leptos::html::Textarea>::new();
    let messages_container_ref = NodeRef::<leptos::html::Div>::new();

    // Theme and Sidebar layout signals
    let (dark_mode, set_dark_mode) = signal(true);
    let (sidebar_open, set_sidebar_open) = signal(false);

    // Monotonic counter for unique message IDs (avoids <For> key collisions).
    let (msg_id_counter, set_msg_id_counter) = signal(0u64);
    let (is_streaming, set_is_streaming) = signal(false);
    // Content of the currently-streaming assistant reply.
    // Kept OUTSIDE the messages Vec so Leptos <For> doesn't miss updates.
    let (streaming_content, set_streaming_content) = signal(String::new());

    // Hydration-only mount effect to read theme preference
    Effect::new(move |_| {
        #[cfg(feature = "hydrate")]
        {
            if let Some(win) = web_sys::window() {
                if let Ok(Some(ls)) = win.local_storage() {
                    if let Ok(Some(val)) = ls.get_item("theme") {
                        set_dark_mode.set(val == "dark");
                    }
                }
            }
        }
    });

    // Auto-scroll messages container to bottom when messages or streaming content updates
    Effect::new(move |_| {
        let _ = messages.get();
        let _ = streaming_content.get();
        #[cfg(feature = "hydrate")]
        {
            if let Some(el) = messages_container_ref.get() {
                request_animation_frame(move || {
                    let scroll_height = el.scroll_height();
                    el.set_scroll_top(scroll_height);
                });
            }
        }
    });

    let toggle_dark_mode = move |_| {
        let next = !dark_mode.get();
        set_dark_mode.set(next);
        #[cfg(feature = "hydrate")]
        {
            if let Some(win) = web_sys::window() {
                if let Ok(Some(ls)) = win.local_storage() {
                    let _ = ls.set_item("theme", if next { "dark" } else { "light" });
                }
            }
        }
    };

    let on_clear = move |_| {
        clear_action.dispatch(ClearMessages {});
        set_messages.set(Vec::new());
        set_msg_id_counter.set(0);
        set_error_text.set(None);
        set_is_streaming.set(false);
        set_streaming_content.set(String::new());
    };

    // Load existing messages from KV store on mount.
    // Guard: only set if the client hasn't already added optimistic messages
    // (prevents a slow resource fetch from overwriting user-typed messages).
    let initial_messages = Resource::new(|| (), |_| get_messages());

    Effect::new(move |_| {
        if let Some(Ok(msgs)) = initial_messages.get() {
            if !msgs.is_empty() {
                set_messages.update(|current| {
                    if current.is_empty() {
                        // Assign sequential IDs to loaded messages
                        let next_id = msgs.len() as u64;
                        *current = msgs
                            .into_iter()
                            .enumerate()
                            .map(|(i, mut m)| {
                                m.id = i as u64;
                                m
                            })
                            .collect();
                        set_msg_id_counter.set(next_id);
                    }
                });
            }
        }
    });

    // We no longer use send_action for browser-side chat turns.
    // Instead we use direct SSE streaming below.

    // Shared send logic — used by both click and Enter key handlers.
    let do_send = move || {
        let text = input_text.get();
        let trimmed = text.trim().to_string();
        if trimmed.is_empty() {
            return;
        }

        // Assign a unique ID from the counter
        let id = msg_id_counter.get_untracked();
        set_msg_id_counter.set(id + 1);

        // Optimistically add user message to UI
        let user_msg = ChatMessage {
            id,
            role: "user".to_string(),
            content: trimmed.clone(),
            timestamp: 0, // Will be set server-side
        };
        set_messages.update(|msgs| msgs.push(user_msg));

        // Clear input
        set_input_text.set(String::new());
        if let Some(el) = textarea_ref.get() {
            el.set_value("");
            #[cfg(feature = "hydrate")]
            {
                use wasm_bindgen::JsCast;
                if let Some(html_el) = el.dyn_ref::<web_sys::HtmlElement>() {
                    let _ = web_sys::HtmlElement::style(html_el).set_property("height", "auto");
                }
            }
        }

        // Clear any previous error
        set_error_text.set(None);

        // Reset streaming content for the new turn
        set_streaming_content.set(String::new());

        // Set streaming state
        set_is_streaming.set(true);

        #[cfg(feature = "hydrate")]
        {
            use wasm_bindgen::prelude::Closure;
            use wasm_bindgen::JsCast;
            use web_sys::{EventSource, MessageEvent};

            let agent_url = get_agent_server_url();
            let encoded_msg = get_agent_server_url_encoded(&trimmed);
            let url = format!("{}/chat/stream?message={}", agent_url, encoded_msg);

            match EventSource::new(&url) {
                Ok(es) => {
                    let trimmed_clone = trimmed.clone();

                    // Token handler — append to the dedicated streaming signal.
                    let on_token = Closure::wrap(Box::new(move |event: web_sys::Event| {
                        if let Ok(msg_event) = event.dyn_into::<MessageEvent>() {
                            if let Some(data) = msg_event.data().as_string() {
                                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&data) {
                                    if let Some(text) = json["text"].as_str() {
                                        set_streaming_content.update(|s| s.push_str(text));
                                    }
                                }
                            }
                        }
                    }) as Box<dyn FnMut(web_sys::Event)>);

                    // Error handler
                    let es_err_clone = es.clone();
                    let on_error = Closure::wrap(Box::new(move |event: web_sys::Event| {
                        es_err_clone.close();
                        set_is_streaming.set(false);
                        if let Ok(msg_event) = event.dyn_into::<MessageEvent>() {
                            if let Some(data) = msg_event.data().as_string() {
                                set_error_text.set(Some(data));
                            } else {
                                set_error_text.set(Some("Error streaming response".to_string()));
                            }
                        } else {
                            set_error_text.set(Some("Connection lost".to_string()));
                        }
                    }) as Box<dyn FnMut(web_sys::Event)>);

                    // Done handler — commit streaming content to the messages vec.
                    let es_done_clone = es.clone();
                    let on_done = Closure::wrap(Box::new(move |_event: web_sys::Event| {
                        es_done_clone.close();

                        // Grab the final accumulated content
                        let final_content = streaming_content.get_untracked();

                        // Commit the completed assistant message to the immutable messages vec
                        let assistant_id = msg_id_counter.get_untracked();
                        set_msg_id_counter.set(assistant_id + 1);
                        let assistant_msg = ChatMessage {
                            id: assistant_id,
                            role: "assistant".to_string(),
                            content: final_content.clone(),
                            timestamp: 0,
                        };
                        set_messages.update(|msgs| msgs.push(assistant_msg));

                        // Clear streaming state
                        set_streaming_content.set(String::new());
                        set_is_streaming.set(false);

                        // Persist to KV
                        let user_content = trimmed_clone.clone();
                        leptos::task::spawn_local(async move {
                            if let Err(e) = save_chat_turn(user_content, final_content).await {
                                eprintln!("Failed to persist message to KV: {:?}", e);
                            }
                        });
                    }) as Box<dyn FnMut(web_sys::Event)>);

                    es.add_event_listener_with_callback("token", on_token.as_ref().unchecked_ref()).unwrap();
                    es.add_event_listener_with_callback("error", on_error.as_ref().unchecked_ref()).unwrap();
                    es.add_event_listener_with_callback("done", on_done.as_ref().unchecked_ref()).unwrap();

                    on_token.forget();
                    on_error.forget();
                    on_done.forget();
                }
                Err(e) => {
                    set_is_streaming.set(false);
                    set_error_text.set(Some(format!("Failed to connect to agent server: {:?}", e)));
                }
            }
        }
    };

    // Button click handler
    let on_send = move |_| do_send();

    // Enter key handler
    let on_keydown = move |ev: leptos::ev::KeyboardEvent| {
        if ev.key() == "Enter" && !ev.shift_key() {
            ev.prevent_default();
            do_send();
        }
    };

    view! {
        <div class=move || if dark_mode.get() {
            "dark h-screen w-screen flex bg-[#212121] text-[#ececec] overflow-hidden"
        } else {
            "h-screen w-screen flex bg-white text-[#0d0d0d] overflow-hidden"
        }>
            // Left Sidebar - permanent on desktop, toggleable overlay on mobile
            <aside class=move || format!(
                "w-64 flex-shrink-0 bg-[#f9f9f9] dark:bg-[#171717] border-r border-[#e5e5e7] dark:border-[#2f2f2f] flex flex-col h-full transition-transform duration-300 z-30 \
                 fixed inset-y-0 left-0 md:static md:translate-x-0 {}",
                if sidebar_open.get() { "translate-x-0" } else { "-translate-x-full" }
            )>
                // Sidebar Header
                <div class="p-3 flex items-center justify-between border-b border-[#e5e5e7] dark:border-[#2f2f2f]">
                    <span class="font-semibold text-sm tracking-tight text-gray-800 dark:text-gray-200">
                        "Antigravity"
                    </span>
                    <button
                        on:click=move |_| set_sidebar_open.set(false)
                        class="md:hidden p-1.5 rounded-lg hover:bg-gray-200 dark:hover:bg-gray-800 text-gray-500 dark:text-gray-400"
                    >
                        // X Icon
                        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                        </svg>
                    </button>
                </div>

                // Sidebar Actions
                <div class="p-3">
                    <button
                        on:click=on_clear
                        class="w-full flex items-center gap-2 px-3 py-2 text-sm font-medium rounded-lg border border-[#e5e5e7] dark:border-[#2f2f2f] bg-white dark:bg-[#212121] hover:bg-gray-50 dark:hover:bg-[#2f2f2f] text-gray-800 dark:text-gray-200 transition-colors"
                    >
                        // Plus icon
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
                        </svg>
                        "New chat"
                    </button>
                </div>

                // Recent Chats Scroll Area
                <div class="flex-1 overflow-y-auto px-3 py-2 space-y-1">
                    <div class="text-xs font-semibold text-gray-400 dark:text-gray-500 px-3 py-2 uppercase tracking-wider">
                        "Recent"
                    </div>
                    <div class="flex items-center gap-2 px-3 py-2 text-sm rounded-lg bg-gray-200/50 dark:bg-gray-800/50 text-gray-900 dark:text-gray-100 font-medium select-none">
                        // Message bubble icon
                        <svg class="w-4 h-4 flex-shrink-0 text-gray-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z" />
                        </svg>
                        <span class="truncate">"Current Chat"</span>
                    </div>
                </div>

                // Sidebar Footer Settings
                <div class="p-3 border-t border-[#e5e5e7] dark:border-[#2f2f2f] space-y-1">
                    // Theme Toggle
                    <button
                        on:click=toggle_dark_mode
                        class="w-full flex items-center gap-3 px-3 py-2.5 text-sm font-medium rounded-lg text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors"
                    >
                        {move || if dark_mode.get() {
                            view! {
                                <svg class="w-4 h-4 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 3v1m0 16v1m9-9h-1M4 12H3m15.364-6.364l-.707.707M6.343 17.657l-.707.707m0-11.314l.707.707m11.314 11.314l.707-.707M12 7a5 5 0 100 10 5 5 0 000-10z" />
                                </svg>
                                <span>"Light theme"</span>
                            }.into_view()
                        } else {
                            view! {
                                <svg class="w-4 h-4 text-gray-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M20.354 15.354A9 9 0 018.646 3.646 9.003 9.003 0 0012 21a9.003 9.003 0 008.354-5.646z" />
                                </svg>
                                <span>"Dark theme"</span>
                            }.into_view()
                        }}
                    </button>

                    // Clear Messages Button
                    <button
                        on:click=on_clear
                        class="w-full flex items-center gap-3 px-3 py-2.5 text-sm font-medium rounded-lg text-red-600 dark:text-red-400 hover:bg-red-50 dark:hover:bg-red-950/20 transition-colors"
                    >
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                        </svg>
                        <span>"Clear conversation"</span>
                    </button>
                </div>
            </aside>

            // Mobile Overlay Backdrop
            <Show when=move || sidebar_open.get()>
                <div
                    on:click=move |_| set_sidebar_open.set(false)
                    class="fixed inset-0 bg-black/40 z-20 md:hidden"
                ></div>
            </Show>

            // Main Chat Area
            <div class="flex-1 flex flex-col h-full overflow-hidden bg-white dark:bg-[#212121] transition-colors duration-200">
                // Top Bar (Header)
                <header class="h-14 border-b border-[#e5e5e7] dark:border-[#2f2f2f] flex items-center px-4 justify-between bg-white dark:bg-[#212121] flex-shrink-0 z-10">
                    <div class="flex items-center gap-2">
                        // Mobile Sidebar Toggle Hamburger
                        <button
                            on:click=move |_| set_sidebar_open.update(|o| *o = !*o)
                            class="md:hidden p-1.5 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-800 text-gray-500 dark:text-gray-400"
                        >
                            <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 12h16M4 18h16" />
                            </svg>
                        </button>

                        <div class="flex items-center gap-1.5 font-semibold text-base text-gray-900 dark:text-gray-100 select-none">
                            "Antigravity Chat"
                            <span class="text-xs font-normal text-gray-400 dark:text-gray-500">
                                "v0.1.4"
                            </span>
                        </div>
                    </div>

                    // Connection / Status indicator
                    <div class="flex items-center gap-2">
                        <div class={move || {
                            if is_streaming.get() {
                                "w-2 h-2 rounded-full bg-[#10a37f] animate-pulse"
                            } else {
                                "w-2 h-2 rounded-full bg-[#10a37f]"
                            }
                        }}></div>
                        <span class="text-xs text-gray-500 dark:text-gray-400 font-medium">
                            {move || {
                                if is_streaming.get() {
                                    "Streaming"
                                } else {
                                    "Ready"
                                }
                            }}
                        </span>
                    </div>
                </header>

                // Messages Viewport
                <div class="flex-1 overflow-y-auto" id="chat-messages-container" node_ref=messages_container_ref>
                    // Empty State Container
                    <Show when=move || messages.get().is_empty() && !is_streaming.get()>
                        <div class="max-w-3xl mx-auto w-full px-4 h-full flex flex-col items-center justify-center text-center py-20">
                            <h2 class="text-[#0d0d0d] dark:text-white text-3xl font-semibold mb-8 tracking-tight">
                                "What's on the agenda today?"
                            </h2>

                            // Prompt Pills
                            <div class="grid grid-cols-1 sm:grid-cols-3 gap-3 w-full max-w-2xl px-4">
                                <button
                                    on:click=move |_| {
                                        set_input_text.set("Explain how async WASM works in Spin using Rust.".to_string());
                                        if let Some(el) = textarea_ref.get() {
                                            el.set_value("Explain how async WASM works in Spin using Rust.");
                                            let _ = el.focus();
                                        }
                                    }
                                    class="p-4 rounded-2xl border border-[#e5e5e7] dark:border-[#2f2f2f] text-left hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors text-sm"
                                >
                                    <div class="font-medium text-gray-800 dark:text-gray-200 mb-1">
                                        "Look something up"
                                    </div>
                                    <div class="text-xs text-gray-500 dark:text-gray-400 line-clamp-2">
                                        "Explain how async WASM works in Spin using Rust."
                                    </div>
                                </button>

                                <button
                                    on:click=move |_| {
                                        set_input_text.set("Write a simple markdown parsing helper function in Rust.".to_string());
                                        if let Some(el) = textarea_ref.get() {
                                            el.set_value("Write a simple markdown parsing helper function in Rust.");
                                            let _ = el.focus();
                                        }
                                    }
                                    class="p-4 rounded-2xl border border-[#e5e5e7] dark:border-[#2f2f2f] text-left hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors text-sm"
                                >
                                    <div class="font-medium text-gray-800 dark:text-gray-200 mb-1">
                                        "Write or edit"
                                    </div>
                                    <div class="text-xs text-gray-500 dark:text-gray-400 line-clamp-2">
                                        "Write a simple markdown parsing helper function in Rust."
                                    </div>
                                </button>

                                <button
                                    on:click=move |_| {
                                        set_input_text.set("Draft a beautiful color palette for dark and light theme styles.".to_string());
                                        if let Some(el) = textarea_ref.get() {
                                            el.set_value("Draft a beautiful color palette for dark and light theme styles.");
                                            let _ = el.focus();
                                        }
                                    }
                                    class="p-4 rounded-2xl border border-[#e5e5e7] dark:border-[#2f2f2f] text-left hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors text-sm"
                                >
                                    <div class="font-medium text-gray-800 dark:text-gray-200 mb-1">
                                        "Create a design"
                                    </div>
                                    <div class="text-xs text-gray-500 dark:text-gray-400 line-clamp-2">
                                        "Draft a beautiful color palette for dark and light theme styles."
                                    </div>
                                </button>
                            </div>
                        </div>
                    </Show>

                    // Active Conversation Messages (aligned to max-w-3xl)
                    <div class="max-w-3xl mx-auto w-full px-4 py-8 flex flex-col gap-8">
                        <For
                            each=move || messages.get()
                            key=|msg| msg.id
                            let:msg
                        >
                            {
                                let is_user = msg.role == "user";
                                let content = msg.content.clone();
                                view! {
                                    <div class=format!("flex w-full gap-4 {}", if is_user { "justify-end" } else { "justify-start" })>
                                        // Avatar Representation
                                        <div class=format!(
                                            "flex-shrink-0 w-8 h-8 rounded-full flex items-center justify-center text-sm font-semibold select-none {}",
                                            if is_user {
                                                "order-2 bg-gray-200 dark:bg-gray-700 text-gray-800 dark:text-gray-200"
                                            } else {
                                                "bg-black dark:bg-white text-white dark:text-black"
                                            }
                                        )>
                                            {if is_user { "U" } else { "A" }}
                                        </div>

                                        // Message Content Box
                                        <div class=format!("flex flex-col max-w-[85%] {}", if is_user { "items-end" } else { "items-start" })>
                                            // Optional sender tag
                                            <div class="text-[11px] text-gray-400 dark:text-gray-500 mb-1 font-medium select-none">
                                                {if is_user { "You" } else { "Agent" }}
                                            </div>

                                            {if is_user {
                                                Either::Left(view! {
                                                    <div class="bg-[#f4f4f4] dark:bg-[#2f2f2f] text-[#0d0d0d] dark:text-[#ececec] rounded-2xl px-4 py-2.5 text-sm leading-relaxed whitespace-pre-wrap">
                                                        {content}
                                                    </div>
                                                })
                                            } else {
                                                let html_content = render_markdown(&content);
                                                Either::Right(view! {
                                                    <div class="markdown-content text-[#0d0d0d] dark:text-[#ececec] leading-relaxed text-sm w-full"
                                                         inner_html=html_content />
                                                })
                                            }}
                                        </div>
                                    </div>
                                }
                            }
                        </For>

                        // Live streaming reply — displayed OUTSIDE <For> so reactive updates work.
                        <Show when=move || is_streaming.get()>
                            <div class="flex w-full gap-4 justify-start">
                                <div class="flex-shrink-0 w-8 h-8 rounded-full flex items-center justify-center text-sm font-semibold bg-black dark:bg-white text-white dark:text-black select-none">
                                    "A"
                                </div>
                                <div class="flex flex-col items-start max-w-[85%]">
                                    <div class="text-[11px] text-gray-400 dark:text-gray-500 mb-1 font-medium select-none">
                                        "Agent"
                                    </div>
                                    {
                                        move || {
                                            let content = streaming_content.get();
                                            if content.is_empty() {
                                                // Still waiting for first token — show bounce dots
                                                view! {
                                                    <div class="text-sm text-gray-500 dark:text-gray-400 flex items-center gap-2">
                                                        <div class="flex gap-1 py-1">
                                                            <div class="w-1.5 h-1.5 bg-[#10a37f] rounded-full animate-bounce" style="animation-delay: 0ms"></div>
                                                            <div class="w-1.5 h-1.5 bg-[#10a37f] rounded-full animate-bounce" style="animation-delay: 150ms"></div>
                                                            <div class="w-1.5 h-1.5 bg-[#10a37f] rounded-full animate-bounce" style="animation-delay: 300ms"></div>
                                                        </div>
                                                    </div>
                                                }.into_any()
                                            } else {
                                                // Tokens arriving — render markdown in real time
                                                let html = render_markdown(&content);
                                                view! {
                                                    <div class="markdown-content text-[#0d0d0d] dark:text-[#ececec] leading-relaxed text-sm w-full"
                                                         inner_html=html />
                                                }.into_any()
                                            }
                                        }
                                    }
                                </div>
                            </div>
                        </Show>

                        // Error toast formatted beautifully
                        <Show when=move || error_text.get().is_some()>
                            <div class="flex w-full gap-4 justify-start">
                                <div class="flex-shrink-0 w-8 h-8 rounded-full flex items-center justify-center text-sm font-semibold bg-red-600 text-white select-none">
                                    "E"
                                </div>
                                <div class="flex flex-col items-start max-w-[85%]">
                                    <div class="text-[11px] text-red-500 mb-1 font-medium select-none">
                                        "Error"
                                    </div>
                                    <div class="bg-red-50 dark:bg-red-950/20 border border-red-200 dark:border-red-900/50 rounded-2xl px-4 py-2.5 text-sm text-red-800 dark:text-red-200 whitespace-pre-wrap">
                                        {move || error_text.get().unwrap_or_default()}
                                    </div>
                                </div>
                            </div>
                        </Show>
                    </div>
                </div>

                // Bottom Input Area (centered max-w-3xl)
                <div class="border-t border-[#e5e5e7] dark:border-[#2f2f2f] bg-white dark:bg-[#212121] py-4">
                    <div class="max-w-3xl mx-auto w-full px-4">
                        <div class="relative flex items-end bg-[#f4f4f4] dark:bg-[#2f2f2f] rounded-3xl p-2 border border-transparent focus-within:border-gray-300 dark:focus-within:border-gray-700 transition-colors">
                            <textarea
                                node_ref=textarea_ref
                                prop:value=move || input_text.get()
                                on:input=move |ev| {
                                    set_input_text.set(event_target_value(&ev));
                                    #[cfg(feature = "hydrate")]
                                    {
                                        use wasm_bindgen::JsCast;
                                        if let Some(target) = ev.target() {
                                            if let Ok(el) = target.dyn_into::<web_sys::HtmlTextAreaElement>() {
                                                if let Some(html_el) = el.dyn_ref::<web_sys::HtmlElement>() {
                                                    let _ = web_sys::HtmlElement::style(html_el).set_property("height", "auto");
                                                    let scroll_height = el.scroll_height();
                                                    let _ = web_sys::HtmlElement::style(html_el).set_property("height", &format!("{scroll_height}px"));
                                                }
                                            }
                                        }
                                    }
                                }
                                on:keydown=on_keydown
                                disabled=move || is_streaming.get()
                                placeholder="Message Antigravity..."
                                rows="1"
                                class="flex-1 bg-transparent resize-none outline-none text-[#0d0d0d] dark:text-[#ececec] placeholder-gray-500 dark:placeholder-gray-400 text-base py-2 px-3 overflow-y-auto max-h-48"
                            ></textarea>
                            <button
                                on:click=on_send
                                disabled=move || is_streaming.get() || input_text.get().trim().is_empty()
                                class="flex items-center justify-center w-8 h-8 rounded-full bg-black dark:bg-[#ececec] text-white dark:text-[#212121] disabled:opacity-20 disabled:cursor-not-allowed hover:opacity-85 active:scale-95 transition-all ml-2 mb-1"
                            >
                                <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" class="w-4 h-4">
                                    <line x1="12" y1="19" x2="12" y2="5"></line>
                                    <polyline points="5 12 12 5 19 12"></polyline>
                                </svg>
                            </button>
                        </div>
                        <div class="text-[11px] text-center text-gray-500 dark:text-gray-400 mt-2 select-none">
                            "Antigravity Chat is powered by Gemini and Spin WASI. Messages are stored locally in KV."
                        </div>
                    </div>
                </div>
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

    view! { <h1 class="text-[#0d0d0d] dark:text-[#ececec] text-center py-20 text-2xl">"Not Found"</h1> }
}

/// Helper: send an HTTP POST request using wasi:http outgoing handler.
///
/// Generic transport function — sends to any HTTP endpoint via `wasi::http/outgoing-handler`.
#[cfg(feature = "ssr")]
fn send_wasi_http_post(
    scheme: &str,
    authority: &str,
    path: &str,
    headers: &[(String, Vec<u8>)],
    body: &[u8],
) -> Result<(u16, Vec<u8>), String> {
    use wasi::http::types::{Fields, Method, OutgoingBody, OutgoingRequest, Scheme};

    let wasi_headers = Fields::from_list(
        &headers
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect::<Vec<_>>(),
    )
    .map_err(|e| format!("Failed to create headers: {e:?}"))?;

    let outgoing_req = OutgoingRequest::new(wasi_headers);
    outgoing_req
        .set_method(&Method::Post)
        .map_err(|_| "Failed to set method".to_string())?;

    let wasi_scheme = if scheme == "https" {
        Scheme::Https
    } else {
        Scheme::Http
    };
    outgoing_req
        .set_scheme(Some(&wasi_scheme))
        .map_err(|_| "Failed to set scheme".to_string())?;
    outgoing_req
        .set_authority(Some(authority))
        .map_err(|_| "Failed to set authority".to_string())?;
    outgoing_req
        .set_path_with_query(Some(path))
        .map_err(|_| "Failed to set path".to_string())?;

    // Write body
    let out_body = outgoing_req
        .body()
        .map_err(|_| "Failed to get body handle".to_string())?;
    {
        let stream = out_body
            .write()
            .map_err(|_| "Failed to get write stream".to_string())?;
        stream
            .blocking_write_and_flush(body)
            .map_err(|e| format!("Failed to write body: {e:?}"))?;
    }

    OutgoingBody::finish(out_body, None)
        .map_err(|e| format!("Failed to finish body: {e:?}"))?;

    let future_response = wasi::http::outgoing_handler::handle(outgoing_req, None)
        .map_err(|e| format!("Failed to send request: {e:?}"))?;

    // Block until response
    let incoming_resp = loop {
        if let Some(result) = future_response.get() {
            break result
                .map_err(|_| "Response already consumed".to_string())?
                .map_err(|e| format!("HTTP error: {e:?}"))?;
        }
        future_response.subscribe().block();
    };

    let status = incoming_resp.status();
    let resp_body_handle = incoming_resp
        .consume()
        .map_err(|_| "Failed to consume body".to_string())?;
    let resp_stream = resp_body_handle
        .stream()
        .map_err(|_| "Failed to get stream".to_string())?;

    let mut resp_bytes = Vec::new();
    loop {
        match resp_stream.blocking_read(65536) {
            Ok(chunk) => resp_bytes.extend_from_slice(&chunk),
            Err(wasi::io::streams::StreamError::Closed) => break,
            Err(e) => return Err(format!("Failed to read response: {e:?}")),
        }
    }

    Ok((status, resp_bytes))
}
/// Send a message via the antigravity-sdk-rust Agent sidecar server (full SDK)
///
/// Calls `POST /chat` on the `agent_server` binary which runs the full SDK with
/// localharness, tools, hooks, and policies.
#[server(prefix = "/api")]
pub async fn send_message(message: String) -> Result<ChatMessage, ServerFnError<String>> {
    let agent_server_url = spin_sdk::variables::get("agent_server_url")
        .unwrap_or_else(|_| "http://127.0.0.1:8080".to_string());

    // Load chat history from KV store
    let store = spin_sdk::key_value::Store::open_default().map_err(|e| e.to_string())?;
    let history: Vec<ChatMessage> = match store.get_json::<Vec<ChatMessage>>("chat_messages") {
        Ok(Some(msgs)) => msgs,
        _ => Vec::new(),
    };

    // ── Sidecar Mode: Full SDK via agent_server ──
    let body = serde_json::to_vec(&serde_json::json!({ "message": message }))
        .map_err(|e| e.to_string())?;

    let headers = vec![("content-type".to_string(), b"application/json".to_vec())];

    // Parse the sidecar URL to extract scheme/authority
    let (scheme, authority) = parse_url_parts(&agent_server_url);

    let (status, resp_bytes) =
        send_wasi_http_post(&scheme, &authority, "/chat", &headers, &body)?;

    if status != 200 {
        let err_text = String::from_utf8_lossy(&resp_bytes);
        return Err(ServerFnError::ServerError(format!(
            "Agent sidecar error ({}): {}",
            status, err_text
        )));
    }

    // Parse sidecar response: { "text": "...", "conversation_id": "..." }
    let resp_json: serde_json::Value =
        serde_json::from_slice(&resp_bytes).map_err(|e| e.to_string())?;

    let text = resp_json["text"]
        .as_str()
        .unwrap_or("(No response)")
        .to_string();

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // Persist messages to KV store with sequential IDs
    let mut messages = history;
    let next_id = messages.iter().map(|m| m.id).max().unwrap_or(0) + 1;

    messages.push(ChatMessage {
        id: next_id,
        role: "user".to_string(),
        content: message,
        timestamp: now.saturating_sub(1),
    });

    let assistant_msg = ChatMessage {
        id: next_id + 1,
        role: "assistant".to_string(),
        content: text,
        timestamp: now,
    };

    messages.push(assistant_msg.clone());

    store
        .set_json("chat_messages", &messages)
        .map_err(|e| ServerFnError::ServerError(e.to_string()))?;

    Ok(assistant_msg)
}

/// Parse a URL string into (scheme, authority) components for `wasi::http`.
#[cfg(feature = "ssr")]
fn parse_url_parts(url: &str) -> (String, String) {
    // Simple URL parsing without pulling in the `url` crate
    let (scheme, rest) = if let Some(stripped) = url.strip_prefix("https://") {
        ("https".to_string(), stripped)
    } else if let Some(stripped) = url.strip_prefix("http://") {
        ("http".to_string(), stripped)
    } else {
        ("http".to_string(), url)
    };

    // Authority is everything up to the first /
    let authority = rest.split('/').next().unwrap_or("127.0.0.1:8080").to_string();

    (scheme, authority)
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

/// Clear all chat messages from the KV store.
#[server(prefix = "/api")]
pub async fn clear_messages() -> Result<(), ServerFnError<String>> {
    let store = spin_sdk::key_value::Store::open_default().map_err(|e| e.to_string())?;
    let _ = store.delete("chat_messages");
    Ok(())
}

/// Parse markdown text into HTML string.
fn render_markdown(markdown: &str) -> String {
    let parser = pulldown_cmark::Parser::new_ext(markdown, pulldown_cmark::Options::all());
    let mut html_output = String::new();
    pulldown_cmark::html::push_html(&mut html_output, parser);
    html_output
}

/// Save a single completed turn (user message & assistant response) to KV.
#[server(prefix = "/api")]
pub async fn save_chat_turn(user_msg: String, assistant_msg: String) -> Result<(), ServerFnError<String>> {
    let store = spin_sdk::key_value::Store::open_default().map_err(|e| e.to_string())?;
    let mut history: Vec<ChatMessage> = match store.get_json::<Vec<ChatMessage>>("chat_messages") {
        Ok(Some(msgs)) => msgs,
        _ => Vec::new(),
    };

    let next_id = history.iter().map(|m| m.id).max().unwrap_or(0) + 1;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    history.push(ChatMessage {
        id: next_id,
        role: "user".to_string(),
        content: user_msg,
        timestamp: now.saturating_sub(1),
    });

    history.push(ChatMessage {
        id: next_id + 1,
        role: "assistant".to_string(),
        content: assistant_msg,
        timestamp: now,
    });

    store
        .set_json("chat_messages", &history)
        .map_err(|e| ServerFnError::ServerError(e.to_string()))?;

    Ok(())
}

#[cfg(feature = "hydrate")]
fn get_agent_server_url() -> String {
    use wasm_bindgen::JsCast;
    if let Some(win) = web_sys::window() {
        if let Some(doc) = win.document() {
            if let Ok(Some(meta)) = doc.query_selector("meta[name='agent-server-url']") {
                if let Ok(meta_el) = meta.dyn_into::<web_sys::HtmlMetaElement>() {
                    return meta_el.content();
                }
            }
        }
    }
    "http://127.0.0.1:8080".to_string()
}

#[cfg(feature = "hydrate")]
fn get_agent_server_url_encoded(val: &str) -> String {
    js_sys::encode_uri_component(val).as_string().unwrap_or_default()
}

