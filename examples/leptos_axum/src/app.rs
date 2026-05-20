use leptos::prelude::*;
use leptos_meta::*;
use leptos_router::{
    components::{Route, Router, Routes},
    *,
};
use leptos::either::Either;

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
                <link rel="preconnect" href="https://fonts.googleapis.com" />
                <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin="true" />
                <link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&display=swap" rel="stylesheet" />
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
    // Server actions
    let send_action = ServerAction::<SendMessage>::new();
    let clear_action = ServerAction::<ClearMessages>::new();

    // Chat messages state
    let (messages, set_messages) = signal(Vec::<ChatMessage>::new());
    let (input_text, set_input_text) = signal(String::new());
    let (error_text, set_error_text) = signal(Option::<String>::None);
    let textarea_ref = NodeRef::<leptos::html::Textarea>::new();

    // Theme and Sidebar layout signals
    let (dark_mode, set_dark_mode) = signal(true);
    let (sidebar_open, set_sidebar_open) = signal(false);

    // Monotonic counter for unique message IDs
    let (msg_id_counter, set_msg_id_counter) = signal(0u64);

    // Hydration-only mount effect to read theme preference
    Effect::new(move |_| {
        #[cfg(feature = "hydrate")]
        {
            leptos::logging::log!("Theme mount effect running...");
            if let Some(win) = web_sys::window() {
                if let Ok(Some(ls)) = win.local_storage() {
                    if let Ok(Some(val)) = ls.get_item("theme") {
                        leptos::logging::log!("Found theme in localStorage: {}", val);
                        set_dark_mode.set(val == "dark");
                    } else {
                        leptos::logging::log!("No theme found in localStorage, default to dark");
                    }
                }
            }
        }
    });

    let toggle_dark_mode = move |_| {
        let next = !dark_mode.get();
        leptos::logging::log!("toggle_dark_mode clicked! Next theme is dark = {}", next);
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
    };

    // Load existing messages on mount
    let initial_messages = Resource::new(|| (), |_| get_messages());

    Effect::new(move |_| {
        if let Some(Ok(msgs)) = initial_messages.get() {
            if !msgs.is_empty() {
                set_messages.update(|current| {
                    if current.is_empty() {
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

    // When server action completes, append the assistant response
    let (handled_version, set_handled_version) = signal(0usize);
    Effect::new(move |_| {
        let v = send_action.version().get();
        let handled = handled_version.get_untracked();
        if v > handled {
            set_handled_version.set(v);
            match send_action.value().get_untracked() {
                Some(Ok(mut assistant_msg)) => {
                    let id = msg_id_counter.get_untracked();
                    set_msg_id_counter.set(id + 1);
                    assistant_msg.id = id;
                    set_messages.update(|msgs| msgs.push(assistant_msg));
                    set_error_text.set(None);
                }
                Some(Err(e)) => {
                    let err = match e {
                        ServerFnError::ServerError(s) => s,
                        other => format!("{other}"),
                    };
                    set_error_text.set(Some(err));
                }
                None => {}
            }
        }
    });

    // Shared send logic
    let do_send = move || {
        let text = input_text.get();
        let trimmed = text.trim().to_string();
        if trimmed.is_empty() {
            return;
        }

        let id = msg_id_counter.get_untracked();
        set_msg_id_counter.set(id + 1);

        let user_msg = ChatMessage {
            id,
            role: "user".to_string(),
            content: trimmed.clone(),
            timestamp: 0,
        };
        set_messages.update(|msgs| msgs.push(user_msg));

        set_input_text.set(String::new());
        if let Some(el) = textarea_ref.get() {
            el.set_value("");
        }

        set_error_text.set(None);
        send_action.dispatch(SendMessage { message: trimmed });
    };

    let on_send = move |_| do_send();

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
            // Left Sidebar
            <aside class=move || format!(
                "w-64 flex-shrink-0 bg-[#f9f9f9] dark:bg-[#171717] border-r border-[#e5e5e7] dark:border-[#2f2f2f] flex flex-col h-full transition-transform duration-300 z-30 \
                 fixed inset-y-0 left-0 md:static md:translate-x-0 {}",
                if sidebar_open.get() { "translate-x-0" } else { "-translate-x-full" }
            )>
                <div class="p-3 flex items-center justify-between border-b border-[#e5e5e7] dark:border-[#2f2f2f]">
                    <span class="font-semibold text-sm tracking-tight text-gray-800 dark:text-gray-200">
                        "Antigravity"
                    </span>
                    <button
                        on:click=move |_| set_sidebar_open.set(false)
                        class="md:hidden p-1.5 rounded-lg hover:bg-gray-200 dark:hover:bg-gray-800 text-gray-500 dark:text-gray-400"
                    >
                        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                        </svg>
                    </button>
                </div>

                <div class="p-3">
                    <button
                        on:click=on_clear
                        class="w-full flex items-center gap-2 px-3 py-2 text-sm font-medium rounded-lg border border-[#e5e5e7] dark:border-[#2f2f2f] bg-white dark:bg-[#212121] hover:bg-gray-50 dark:hover:bg-[#2f2f2f] text-gray-800 dark:text-gray-200 transition-colors"
                    >
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
                        </svg>
                        "New chat"
                    </button>
                </div>

                <div class="flex-1 overflow-y-auto px-3 py-2 space-y-1">
                    <div class="text-xs font-semibold text-gray-400 dark:text-gray-500 px-3 py-2 uppercase tracking-wider">
                        "Recent"
                    </div>
                    <div class="flex items-center gap-2 px-3 py-2 text-sm rounded-lg bg-gray-200/50 dark:bg-gray-800/50 text-gray-900 dark:text-gray-100 font-medium select-none">
                        <svg class="w-4 h-4 flex-shrink-0 text-gray-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z" />
                        </svg>
                        <span class="truncate">"Current Chat"</span>
                    </div>
                </div>

                <div class="p-3 border-t border-[#e5e5e7] dark:border-[#2f2f2f] space-y-1">
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

            <Show when=move || sidebar_open.get()>
                <div
                    on:click=move |_| set_sidebar_open.set(false)
                    class="fixed inset-0 bg-black/40 z-20 md:hidden"
                ></div>
            </Show>

            // Main Chat Area
            <div class="flex-1 flex flex-col h-full overflow-hidden bg-white dark:bg-[#212121] transition-colors duration-200">
                <header class="h-14 border-b border-[#e5e5e7] dark:border-[#2f2f2f] flex items-center px-4 justify-between bg-white dark:bg-[#212121] flex-shrink-0 z-10">
                    <div class="flex items-center gap-2">
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

                    <div class="flex items-center gap-2">
                        <div class={move || {
                            if send_action.pending().get() {
                                "w-2 h-2 rounded-full bg-[#10a37f] animate-pulse"
                            } else {
                                "w-2 h-2 rounded-full bg-[#10a37f]"
                            }
                        }}></div>
                        <span class="text-xs text-gray-500 dark:text-gray-400 font-medium">
                            {move || {
                                if send_action.pending().get() {
                                    "Thinking"
                                } else {
                                    "Ready"
                                }
                            }}
                        </span>
                    </div>
                </header>

                <div class="flex-1 overflow-y-auto" id="chat-messages-container">
                    <Show when=move || messages.get().is_empty() && !send_action.pending().get()>
                        <div class="max-w-3xl mx-auto w-full px-4 h-full flex flex-col items-center justify-center text-center py-20">
                            <h2 class="text-[#0d0d0d] dark:text-white text-3xl font-semibold mb-8 tracking-tight">
                                "What's on the agenda today?"
                            </h2>

                            <div class="grid grid-cols-1 sm:grid-cols-3 gap-3 w-full max-w-2xl px-4">
                                <button
                                    on:click=move |_| {
                                        set_input_text.set("Explain how async works in Rust's futures mechanism.".to_string());
                                        if let Some(el) = textarea_ref.get() {
                                            el.set_value("Explain how async works in Rust's futures mechanism.");
                                            let _ = el.focus();
                                        }
                                    }
                                    class="p-4 rounded-2xl border border-[#e5e5e7] dark:border-[#2f2f2f] text-left hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors text-sm"
                                >
                                    <div class="font-medium text-gray-800 dark:text-gray-200 mb-1">
                                        "Look something up"
                                    </div>
                                    <div class="text-xs text-gray-500 dark:text-gray-400 line-clamp-2">
                                        "Explain how async works in Rust's futures mechanism."
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

                                        <div class=format!("flex flex-col max-w-[85%] {}", if is_user { "items-end" } else { "items-start" })>
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

                        <Show when=move || send_action.pending().get()>
                            <div class="flex w-full gap-4 justify-start">
                                <div class="flex-shrink-0 w-8 h-8 rounded-full flex items-center justify-center text-sm font-semibold bg-black dark:bg-white text-white dark:text-black select-none">
                                    "A"
                                </div>
                                <div class="flex flex-col items-start max-w-[85%]">
                                    <div class="text-[11px] text-gray-400 dark:text-gray-500 mb-1 font-medium select-none">
                                        "Agent"
                                    </div>
                                    <div class="text-sm text-gray-500 dark:text-gray-400 flex items-center gap-2">
                                        <div class="flex gap-1 py-1">
                                            <div class="w-1.5 h-1.5 bg-[#10a37f] rounded-full animate-bounce" style="animation-delay: 0ms"></div>
                                            <div class="w-1.5 h-1.5 bg-[#10a37f] rounded-full animate-bounce" style="animation-delay: 150ms"></div>
                                            <div class="w-1.5 h-1.5 bg-[#10a37f] rounded-full animate-bounce" style="animation-delay: 300ms"></div>
                                        </div>
                                    </div>
                                </div>
                            </div>
                        </Show>

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

                <div class="border-t border-[#e5e5e7] dark:border-[#2f2f2f] bg-white dark:bg-[#212121] py-4">
                    <div class="max-w-3xl mx-auto w-full px-4">
                        <div class="relative flex items-center bg-[#f4f4f4] dark:bg-[#2f2f2f] rounded-2xl p-1.5 border border-transparent focus-within:border-gray-300 dark:focus-within:border-gray-700 transition-colors">
                            <textarea
                                node_ref=textarea_ref
                                prop:value=move || input_text.get()
                                on:input=move |ev| set_input_text.set(event_target_value(&ev))
                                on:keydown=on_keydown
                                disabled=move || send_action.pending().get()
                                placeholder="Message Antigravity..."
                                rows="1"
                                class="flex-1 bg-transparent resize-none outline-none text-[#0d0d0d] dark:text-[#ececec] placeholder-gray-500 dark:placeholder-gray-400 text-base py-2 px-3 overflow-y-auto max-h-48"
                            ></textarea>
                            <button
                                on:click=on_send
                                disabled=move || send_action.pending().get() || input_text.get().trim().is_empty()
                                class="flex items-center justify-center w-8 h-8 rounded-xl bg-black dark:bg-[#ececec] text-white dark:text-[#212121] disabled:opacity-20 disabled:cursor-not-allowed hover:opacity-85 active:scale-95 transition-all ml-2"
                            >
                                <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor" class="w-4 h-4">
                                    <path d="M3.478 2.404a.75.75 0 0 0-.926.941l2.432 7.905H13.5a.75.75 0 0 1 0 1.5H4.984l-2.432 7.905a.75.75 0 0 0 .926.94 60.53 60.53 0 0 0 18.425-7.706a.75.75 0 0 0 0-1.238L3.478 2.404Z" />
                                </svg>
                            </button>
                        </div>
                        <div class="text-[11px] text-center text-gray-500 dark:text-gray-400 mt-2 select-none">
                            "Antigravity Chat is powered by Gemini and Axum. Messages are stored in session memory."
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
        if let Some(resp) = use_context::<leptos_axum::ResponseOptions>() {
            resp.set_status(http::StatusCode::NOT_FOUND);
        }
    }

    view! {
        <div class="min-h-screen flex items-center justify-center bg-white dark:bg-[#212121] text-[#0d0d0d] dark:text-[#ececec]">
            <div class="text-center">
                <h1 class="text-6xl font-bold text-red-500 mb-4">"404"</h1>
                <p class="text-lg mb-4">"Page not found"</p>
                <a href="/" class="text-[#10a37f] hover:underline">"Go back home"</a>
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

    let id = {
        let history = chat_history_state.lock().await;
        history.len() as u64
    };

    let user_msg = ChatMessage {
        id,
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
        id: id + 1,
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

/// Clear all chat messages from the in-memory history.
#[server(prefix = "/api")]
pub async fn clear_messages() -> Result<(), ServerFnError<String>> {
    use crate::ChatHistoryState;

    let chat_history_state =
        use_context::<ChatHistoryState>().ok_or_else(|| "Chat history not available".to_string())?;

    let mut history = chat_history_state.lock().await;
    history.clear();
    Ok(())
}

/// Parse markdown text into HTML string.
fn render_markdown(markdown: &str) -> String {
    let parser = pulldown_cmark::Parser::new(markdown);
    let mut html_output = String::new();
    pulldown_cmark::html::push_html(&mut html_output, parser);
    html_output
}
