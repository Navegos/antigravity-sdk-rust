//! Direct Gemini API Client — No localharness required.
//!
//! This module provides a lightweight, transport-agnostic client for the Gemini REST API.
//! Unlike the full [`Agent`](crate::agent::Agent) which communicates with `localharness` via
//! WebSocket, `GeminiDirectClient` builds HTTP request payloads and parses responses directly.
//!
//! The actual HTTP transport is intentionally left to the caller, making this module compatible
//! with any HTTP client: `reqwest`, `wasi::http`, `hyper`, etc.
//!
//! # Example
//!
//! ```no_run
//! use antigravity_sdk_rust::direct::{GeminiDirectClient, ChatEntry};
//! use antigravity_sdk_rust::types::GeminiConfig;
//!
//! let config = GeminiConfig::default();
//! let client = GeminiDirectClient::new(&config)
//!     .with_system_instruction("You are a helpful assistant.".to_string());
//!
//! // Build a request
//! let request = client.build_request("your-api-key", "Hello!", &[]).unwrap();
//!
//! // Send via your preferred HTTP client (reqwest, wasi::http, etc.)
//! // let response_bytes = your_http_client.post(&request.url, &request.headers, &request.body)?;
//!
//! // Parse the response
//! // let text = GeminiDirectClient::parse_response(&response_bytes).unwrap();
//! ```

use crate::types::{GeminiConfig, SystemInstructions};
use serde::{Deserialize, Serialize};
use std::fmt::Write;

/// A single entry in a conversation history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatEntry {
    /// The role: `"user"` or `"model"` (Gemini uses `"model"` for assistant messages).
    pub role: String,
    /// The text content of the message.
    pub content: String,
}

/// A fully-formed HTTP request ready to be sent to the Gemini API.
#[derive(Debug, Clone)]
pub struct GeminiRequest {
    /// The full URL to send the request to (e.g. `https://generativelanguage.googleapis.com/v1beta/models/gemini-3.5-flash:generateContent`).
    pub url: String,
    /// The HTTPS scheme component (for `wasi::http` callers).
    pub scheme: String,
    /// The authority/host component (for `wasi::http` callers).
    pub authority: String,
    /// The path + query component (for `wasi::http` callers).
    pub path: String,
    /// HTTP headers to include (name, value as bytes).
    pub headers: Vec<(String, Vec<u8>)>,
    /// The JSON request body as bytes.
    pub body: Vec<u8>,
}

/// A lightweight, transport-agnostic Gemini API client.
///
/// This client builds HTTP request payloads and parses Gemini REST API responses.
/// It does **not** perform any network I/O — the caller provides the HTTP transport.
///
/// This is useful for environments where TCP/WebSocket is unavailable (e.g. Spin/WASI)
/// or when you want to use the Gemini API directly without `localharness`.
#[derive(Debug, Clone)]
pub struct GeminiDirectClient {
    model: String,
    system_instruction: Option<String>,
    temperature: f64,
    max_output_tokens: u32,
}

impl GeminiDirectClient {
    /// Creates a new `GeminiDirectClient` from a [`GeminiConfig`].
    ///
    /// Uses the default model name from the config, or falls back to `gemini-3.5-flash`.
    pub fn new(config: &GeminiConfig) -> Self {
        Self {
            model: config.models.default.name.clone(),
            system_instruction: None,
            temperature: 0.7,
            max_output_tokens: 2048,
        }
    }

    /// Creates a new `GeminiDirectClient` with explicit model name.
    pub fn with_model(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            system_instruction: None,
            temperature: 0.7,
            max_output_tokens: 2048,
        }
    }

    /// Sets a system instruction for the client. Returns `self` for chaining.
    pub fn with_system_instruction(mut self, instruction: String) -> Self {
        self.system_instruction = Some(instruction);
        self
    }

    /// Sets the system instruction from SDK [`SystemInstructions`]. Returns `self` for chaining.
    pub fn with_system_instructions(mut self, instructions: &SystemInstructions) -> Self {
        match instructions {
            SystemInstructions::Custom(custom) => {
                self.system_instruction = Some(custom.text.clone());
            }
            SystemInstructions::Appended(appended) => {
                let mut text = String::new();
                if let Some(ref identity) = appended.custom_identity {
                    text.push_str(identity);
                    text.push('\n');
                }
                for section in &appended.appended_sections {
                    let _ = write!(text, "## {}\n{}\n\n", section.title, section.content);
                }
                self.system_instruction = Some(text);
            }
        }
        self
    }

    /// Sets the temperature for generation. Returns `self` for chaining.
    pub const fn with_temperature(mut self, temperature: f64) -> Self {
        self.temperature = temperature;
        self
    }

    /// Sets the maximum output tokens. Returns `self` for chaining.
    pub const fn with_max_output_tokens(mut self, max_tokens: u32) -> Self {
        self.max_output_tokens = max_tokens;
        self
    }

    /// Returns the configured model name.
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Builds a [`GeminiRequest`] for the given message and conversation history.
    ///
    /// The API key is passed as a header (`x-goog-api-key`) rather than in the URL
    /// to avoid issues with WASI path validators rejecting special characters.
    ///
    /// # Arguments
    ///
    /// * `api_key` — The Gemini API key.
    /// * `message` — The new user message to send.
    /// * `history` — Previous conversation entries (role + content).
    ///
    /// # Errors
    ///
    /// Returns an error if the request body cannot be serialized.
    pub fn build_request(
        &self,
        api_key: &str,
        message: &str,
        history: &[ChatEntry],
    ) -> Result<GeminiRequest, anyhow::Error> {
        let authority = "generativelanguage.googleapis.com";
        let path = format!("/v1beta/models/{}:generateContent", self.model);
        let url = format!("https://{authority}{path}");

        // Build conversation contents
        let mut contents = Vec::new();
        for entry in history {
            contents.push(serde_json::json!({
                "role": entry.role,
                "parts": [{ "text": entry.content }]
            }));
        }
        contents.push(serde_json::json!({
            "role": "user",
            "parts": [{ "text": message }]
        }));

        let mut request_body = serde_json::json!({
            "contents": contents,
            "generationConfig": {
                "temperature": self.temperature,
                "maxOutputTokens": self.max_output_tokens
            }
        });

        // Add system instruction if configured
        if let Some(ref instruction) = self.system_instruction {
            request_body["systemInstruction"] = serde_json::json!({
                "parts": [{ "text": instruction }]
            });
        }

        let body = serde_json::to_vec(&request_body)?;

        let headers = vec![
            ("content-type".to_string(), b"application/json".to_vec()),
            ("x-goog-api-key".to_string(), api_key.as_bytes().to_vec()),
        ];

        Ok(GeminiRequest {
            url,
            scheme: "https".to_string(),
            authority: authority.to_string(),
            path,
            headers,
            body,
        })
    }

    /// Parses a Gemini API JSON response body and extracts the generated text.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The response body cannot be parsed as JSON.
    /// - The expected `candidates[0].content.parts[0].text` path is missing.
    pub fn parse_response(response_body: &[u8]) -> Result<String, anyhow::Error> {
        let json: serde_json::Value = serde_json::from_slice(response_body)?;

        let text = json["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Unexpected Gemini response structure: {}",
                    String::from_utf8_lossy(&response_body[..response_body.len().min(500)])
                )
            })?
            .to_string();

        Ok(text)
    }

    /// Parses a Gemini API error response and returns a descriptive error message.
    pub fn parse_error(status: u16, response_body: &[u8]) -> String {
        let body_text = String::from_utf8_lossy(response_body);
        format!("Gemini API error ({}): {}", status, body_text)
    }

    /// Converts a slice of [`ChatEntry`] items to the Gemini API `contents` JSON format.
    pub fn entries_to_contents(history: &[ChatEntry], new_message: &str) -> serde_json::Value {
        let mut contents = Vec::new();
        for entry in history {
            contents.push(serde_json::json!({
                "role": entry.role,
                "parts": [{ "text": entry.content }]
            }));
        }
        contents.push(serde_json::json!({
            "role": "user",
            "parts": [{ "text": new_message }]
        }));
        serde_json::Value::Array(contents)
    }
}
