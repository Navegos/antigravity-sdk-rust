# Examples

This directory contains examples demonstrating how to use the Google Antigravity Rust SDK.

## Getting Started

### 1. Hello World Example (`hello_world.rs`)

A simple console application that configures the agent, launches the mock localharness server, sends a user message, streams the response, and exits cleanly.

To run this example:
```sh
cargo run --example hello_world
```

---

## Web Integration Examples

The SDK provides two web integration patterns depending on your deployment target:

| Example | Runtime | SDK Integration | Use Case |
|---------|---------|-----------------|----------|
| [`leptos_axum`](#2-leptos--axum-native) | Native (Tokio) | Full Agent SDK | Standard web servers (VPS, Docker, bare metal) |
| [`leptos_ssr_axum`](#3-leptos--spin-wasi) + [`agent_server`](#4-agent-server-sidecar) | Spin/WASI | Full Agent SDK (via sidecar) | Edge/serverless (Akamai, SpinKube) |

---

### 2. Leptos + Axum (Native)

**Path:** `leptos_axum/`

A full-featured chat application using [Leptos](https://leptos.dev) (v0.8) with native [Axum](https://github.com/tokio-rs/axum) as the web server. The Agent runs in-process — no sidecar needed.

**Prerequisites:**
- `localharness` binary installed (`./scripts/install_harness.sh`)
- `GEMINI_API_KEY` environment variable set
- [`cargo-leptos`](https://github.com/leptos-rs/cargo-leptos) installed

**Run:**
```sh
cd examples/leptos_axum
echo "GEMINI_API_KEY=your-key" > .env
cargo leptos serve
```
Open `http://localhost:3000` in your browser.

**Architecture:**
```
Browser → Axum Server (native)
               ├─ Agent::chat()
               ├─ WebSocket → localharness
               └─ Gemini API
```

---

### 3. Leptos + Spin/WASI

**Path:** `leptos_ssr_axum/`

A chat application that runs as a [Spin](https://github.com/spinframework/spin) WebAssembly component (integrating Leptos with Spin via [leptos-spin](https://github.com/akamai-developers/leptos-spin)). Since Spin components cannot make outbound TCP/WebSocket connections (only `wasi:http`), this example runs in **Sidecar Mode** using the [`agent_server`](#4-agent-server-sidecar) sidecar to get full SDK features (harness, tools, hooks, and policies).

```sh
# Terminal 1: Start the sidecar (runs full SDK + localharness)
cd examples/agent_server
GEMINI_API_KEY=your-key cargo run

# Terminal 2: Start Spin
cd examples/leptos_ssr_axum
spin build --up
```

**Architecture:**
```
Browser → Spin Component (WASI)
               │
               ├─ POST /chat ──→ agent_server (native sidecar)
               │                      ├─ Agent::chat()
               │                      ├─ WebSocket → localharness
               │                      └─ Gemini API
               │
               └─ KV Store (chat history)
```

> **Note**: While standard Spin components are restricted to `wasi:http` for outbound traffic (necessitating sidecar mode), the SDK compiles directly to the `wasm32-wasip1` target. In WASI environments that permit raw outbound TCP sockets, the SDK can connect directly to the host-side harness server via WebSockets using `WasmConnectionStrategy`.

---

### 4. Agent Server (Sidecar)

**Path:** `agent_server/`

A lightweight HTTP server that wraps the full `antigravity-sdk-rust` Agent and exposes it via REST. Designed as a companion sidecar for Spin/WASI applications that cannot use TCP/WebSocket directly.

**Endpoints:**

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/chat` | Send a message, get a response via the full Agent pipeline |
| `GET` | `/health` | Health check |

**Environment Variables:**

| Variable | Default | Description |
|----------|---------|-------------|
| `GEMINI_API_KEY` | (required) | Your Gemini API key |
| `AGENT_SERVER_PORT` | `8080` | Port the sidecar listens on |
| `GEMINI_MODEL` | `gemini-3.5-flash` | Model to use |
| `ANTIGRAVITY_HARNESS_PATH` | `bin/localharness` | Path to localharness binary |

**Run:**
```sh
cd examples/agent_server
GEMINI_API_KEY=your-key cargo run
```

**Test with curl:**
```sh
curl -X POST http://127.0.0.1:8080/chat \
  -H "Content-Type: application/json" \
  -d '{"message": "Hello, what can you do?"}'
```
