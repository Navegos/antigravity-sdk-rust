use leptos::prelude::*;
use leptos_meta::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatMessage {
    pub is_user: bool,
    pub text: String,
}

#[server(SendMessage)]
pub async fn send_message(
    message: String,
    conversation_id: String,
) -> Result<String, ServerFnError> {
    use antigravity_sdk_rust::agent::{Agent, AgentConfig};
    use antigravity_sdk_rust::policy;

    let mut config = AgentConfig::default();
    config.conversation_id = Some(conversation_id.clone());

    // Persist conversation databases in target/
    config.save_dir = Some("target/agent_store".to_string());

    // Apply default security policies
    config.policies = Some(vec![policy::allow_all()]);

    // Start the Agent and execute the session
    let mut agent = Agent::new(config);
    agent
        .start()
        .await
        .map_err(|e| ServerFnError::new(format!("Harness start failed: {e}")))?;

    let response = agent
        .chat(&message)
        .await
        .map_err(|e| ServerFnError::new(format!("Agent communication error: {e}")))?;

    let _ = agent.stop().await;

    Ok(response.text)
}

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

    view! {
        <Stylesheet id="leptos" href="/pkg/leptos_ssr_axum.css" />
        <Title text="Antigravity Agent Interface" />
        <main>
            <ChatInterface />
        </main>
    }
}

#[component]
pub fn ChatInterface() -> impl IntoView {
    // Unique conversation ID for this session
    let conversation_id = "leptos-ssr-demo".to_string();

    let (messages, set_messages) = signal(vec![
        ChatMessage {
            is_user: false,
            text: "Welcome to the Google Antigravity Agent Chat! I can execute shell commands, edit files, and run tasks directly. Try asking me to run a command or write a file.".to_string(),
        }
    ]);

    let (input_text, set_input_text) = signal(String::new());
    let send_action = ServerAction::<SendMessage>::new();

    // Triggered when user submits a message
    let on_submit = move |ev: leptos::web_sys::SubmitEvent| {
        ev.prevent_default();
        let prompt = input_text.get().trim().to_string();
        if prompt.is_empty() {
            return;
        }

        // Optimistically add user's prompt
        set_messages.update(|msgs| {
            msgs.push(ChatMessage {
                is_user: true,
                text: prompt.clone(),
            });
        });

        // Clear input box
        set_input_text.set(String::new());

        // Dispatch server action
        send_action.dispatch(SendMessage {
            message: prompt,
            conversation_id: conversation_id.clone(),
        });
    };

    // Watch for response from the server action
    let value = send_action.value();
    Effect::new(move |_| {
        if let Some(res) = value.get() {
            match res {
                Ok(reply) => {
                    set_messages.update(|msgs| {
                        msgs.push(ChatMessage {
                            is_user: false,
                            text: reply,
                        });
                    });
                }
                Err(err) => {
                    set_messages.update(|msgs| {
                        msgs.push(ChatMessage {
                            is_user: false,
                            text: format!("Error: {}", err),
                        });
                    });
                }
            }
        }
    });

    let is_pending = send_action.pending();

    view! {
        <style>
            r#"
            body {
                background: radial-gradient(circle at top right, #1a1c29 0%, #0d0e15 100%);
                min-height: 100vh;
                display: flex;
                align-items: center;
                justify-content: center;
                padding: 20px;
                box-sizing: border-box;
            }

            .chat-container {
                width: 100%;
                max-width: 800px;
                height: 80vh;
                background: rgba(22, 24, 37, 0.7);
                backdrop-filter: blur(20px);
                -webkit-backdrop-filter: blur(20px);
                border: 1px solid rgba(255, 255, 255, 0.08);
                border-radius: 24px;
                box-shadow: 0 20px 50px rgba(0, 0, 0, 0.4);
                display: flex;
                flex-direction: column;
                overflow: hidden;
                transition: transform 0.3s ease, box-shadow 0.3s ease;
            }

            .chat-header {
                padding: 24px;
                background: rgba(255, 255, 255, 0.02);
                border-bottom: 1px solid rgba(255, 255, 255, 0.06);
                display: flex;
                align-items: center;
                gap: 16px;
            }

            .header-dot {
                width: 12px;
                height: 12px;
                background: #6366f1;
                border-radius: 50%;
                box-shadow: 0 0 12px #6366f1;
                animation: glow 2s infinite ease-in-out;
            }

            .header-title {
                font-size: 1.25rem;
                font-weight: 600;
                background: linear-gradient(to right, #ffffff, #a5b4fc);
                -webkit-background-clip: text;
                -webkit-text-fill-color: transparent;
                margin: 0;
            }

            .messages-list {
                flex: 1;
                padding: 24px;
                overflow-y: auto;
                display: flex;
                flex-direction: column;
                gap: 20px;
            }

            .message-wrapper {
                display: flex;
                width: 100%;
            }

            .message-wrapper.user {
                justify-content: flex-end;
            }

            .message-wrapper.assistant {
                justify-content: flex-start;
            }

            .message-bubble {
                max-width: 75%;
                padding: 16px 20px;
                border-radius: 18px;
                line-height: 1.5;
                font-size: 0.95rem;
                white-space: pre-wrap;
                word-break: break-word;
                animation: popIn 0.3s cubic-bezier(0.16, 1, 0.3, 1);
            }

            .message-wrapper.user .message-bubble {
                background: linear-gradient(135deg, #4f46e5 0%, #3730a3 100%);
                color: #ffffff;
                border-bottom-right-radius: 4px;
                box-shadow: 0 4px 15px rgba(79, 70, 229, 0.3);
            }

            .message-wrapper.assistant .message-bubble {
                background: rgba(255, 255, 255, 0.05);
                color: #e2e8f0;
                border-bottom-left-radius: 4px;
                border: 1px solid rgba(255, 255, 255, 0.04);
            }

            .typing-indicator {
                display: flex;
                gap: 6px;
                padding: 16px 20px;
                background: rgba(255, 255, 255, 0.03);
                border-radius: 18px;
                border-bottom-left-radius: 4px;
                width: fit-content;
                animation: popIn 0.3s ease;
            }

            .typing-dot {
                width: 8px;
                height: 8px;
                background: #a5b4fc;
                border-radius: 50%;
                animation: bounce 1.4s infinite ease-in-out both;
            }

            .typing-dot:nth-child(1) { animation-delay: -0.32s; }
            .typing-dot:nth-child(2) { animation-delay: -0.16s; }

            .input-form {
                padding: 20px 24px;
                background: rgba(255, 255, 255, 0.01);
                border-top: 1px solid rgba(255, 255, 255, 0.06);
                display: flex;
                gap: 12px;
            }

            .chat-input {
                flex: 1;
                background: rgba(255, 255, 255, 0.04);
                border: 1px solid rgba(255, 255, 255, 0.08);
                border-radius: 14px;
                padding: 16px 20px;
                color: #ffffff;
                font-family: inherit;
                font-size: 0.95rem;
                outline: none;
                transition: border-color 0.2s ease, background-color 0.2s ease;
            }

            .chat-input:focus {
                border-color: #6366f1;
                background: rgba(255, 255, 255, 0.06);
            }

            .send-button {
                background: linear-gradient(135deg, #6366f1 0%, #4f46e5 100%);
                color: #ffffff;
                border: none;
                border-radius: 14px;
                padding: 0 28px;
                font-weight: 600;
                font-family: inherit;
                font-size: 0.95rem;
                cursor: pointer;
                transition: transform 0.1s ease, box-shadow 0.2s ease;
                display: flex;
                align-items: center;
                justify-content: center;
            }

            .send-button:hover {
                box-shadow: 0 0 20px rgba(99, 102, 241, 0.4);
            }

            .send-button:active {
                transform: scale(0.97);
            }

            @keyframes glow {
                0%, 100% { opacity: 0.6; box-shadow: 0 0 8px #6366f1; }
                50% { opacity: 1; box-shadow: 0 0 16px #818cf8; }
            }

            @keyframes popIn {
                from { opacity: 0; transform: translateY(8px) scale(0.98); }
                to { opacity: 1; transform: translateY(0) scale(1); }
            }

            @keyframes bounce {
                0%, 80%, 100% { transform: scale(0); }
                40% { transform: scale(1); }
            }
            "#
        </style>

        <div class="chat-container">
            <div class="chat-header">
                <div class="header-dot"></div>
                <h1 class="header-title">Antigravity SDK Assistant</h1>
            </div>

            <div class="messages-list">
                <For
                    each=move || messages.get()
                    key=|msg| msg.text.clone()
                    let:msg
                >
                    <div class=move || if msg.is_user { "message-wrapper user" } else { "message-wrapper assistant" }>
                        <div class="message-bubble">
                            {msg.text}
                        </div>
                    </div>
                </For>

                <Show when=move || is_pending.get()>
                    <div class="message-wrapper assistant">
                        <div class="typing-indicator">
                            <div class="typing-dot"></div>
                            <div class="typing-dot"></div>
                            <div class="typing-dot"></div>
                        </div>
                    </div>
                </Show>
            </div>

            <form class="input-form" on:submit=on_submit>
                <input
                    type="text"
                    class="chat-input"
                    placeholder="Ask the agent to write a file or run a command..."
                    prop:value=move || input_text.get()
                    on:input=move |ev| set_input_text.set(event_target_value(&ev))
                    disabled=move || is_pending.get()
                />
                <button type="submit" class="send-button" disabled=move || is_pending.get()>
                    "Send"
                </button>
            </form>
        </div>
    }
}
