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

## WebAssembly and Frontend Integration

### 2. Leptos WebAssembly Integration (`leptos_wasm_test/`)

A sample single-page web application using the [Leptos](https://leptos.dev) framework (v0.8) compiled to WebAssembly target (`wasm32-unknown-unknown`). It integrates `antigravity-sdk-rust` and implements a custom frontend browser-compatible `Connection` strategy to verify WASM compilation parity.

To check compilation of the WASM target:
```sh
cd leptos_wasm_test
cargo check --target wasm32-unknown-unknown
```

---

### 3. Leptos Server-Side Rendering (SSR) Axum Integration (`leptos_ssr_axum/`)

A comprehensive sample application using [Leptos](https://leptos.dev) (v0.8) and [Axum](https://github.com/tokio-rs/axum) demonstrating Server-Side Rendering (SSR) with full hydration. The backend native Axum server integrates with the `antigravity-sdk-rust` and spawns the `localharness` subprocess natively to orchestrate stateful conversation sessions.

To run this example locally:
1. Ensure the `localharness` binary is installed (e.g., using the `scripts/install_harness.sh` script).
2. Set your `GEMINI_API_KEY` environment variable.
3. Start the server:
   ```sh
   cd leptos_ssr_axum
   cargo run --features ssr
   ```
4. Navigate to `http://localhost:3000` in your web browser.

