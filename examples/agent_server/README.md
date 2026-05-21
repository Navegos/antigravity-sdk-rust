# Agent Server (Sidecar Proxy)

This directory contains `agent_server`, a lightweight, native HTTP sidecar server that wraps the `antigravity-sdk-rust` Agent. It exposes a REST/SSE (Server-Sent Events) API to bridge runtime limitations for environments like WebAssembly/WASI (e.g., [Spin](https://github.com/spinframework/spin) or [SpinKube](https://spinkube.dev/)).

## Why is a Sidecar Needed?

WebAssembly guest components targeting `wasi:http` are constrained by sandboxing capabilities. Most standard WASI runtimes do not currently support raw outbound TCP/WebSocket sockets directly from the guest. Since the Antigravity SDK communicates with the host-side `localharness` server via WebSockets, a WebAssembly guest cannot interact with it directly.

To work around this limitation:
1. **Sidecar Process**: The `agent_server` runs as a native host-side process alongside the WASI guest.
2. **Lifecycle Management**: It spins up the `localharness` subprocess, executes tool calls, monitors policies, and drives the agent conversation loop.
3. **HTTP/SSE API**: The WASI guest communicates with the sidecar using standard HTTP/SSE requests.

---

## Prerequisites

- **Localharness**: Ensure you have the `localharness` binary installed at the path specified by `ANTIGRAVITY_HARNESS_PATH` (defaults to `bin/localharness` in the workspace root).
- **Gemini API Key**: A valid Gemini API key set in your environment.

---

## Environment Variables

Configure the server using the following environment variables (or by placing them in a `.env` file inside this directory):

| Variable | Default | Description |
|----------|---------|-------------|
| `GEMINI_API_KEY` | *(Required)* | Your Google Gemini API Key |
| `AGENT_SERVER_PORT` | `8080` | The port the sidecar listens on |
| `GEMINI_MODEL` | `gemini-2.5-flash` | The Gemini model used by the agent |
| `ANTIGRAVITY_HARNESS_PATH` | `bin/localharness` | Path to the localharness binary relative to workspace |
| `RUST_LOG` | `info` | Logging verbosity level (`debug`, `info`, `warn`, `error`) |

---

## Running the Server

To start the sidecar server locally:

```sh
# From the workspace root
cd examples/agent_server
export GEMINI_API_KEY="your-api-key"
cargo run
```

---

## Endpoint Reference

### 1. `GET /chat/stream`
Starts a Server-Sent Events (SSE) chat stream for a session.

- **Query Parameters**:
  - `session_id` (String): A unique identifier for the conversation session.
  - `message` (String): The prompt to send to the agent.
- **Example curl**:
  ```sh
  curl -N "http://127.0.0.1:8080/chat/stream?session_id=sess_123&message=hello"
  ```

### 2. `POST /halt`
Terminates a running agent execution stream for a given session.

- **Request Body**:
  ```json
  {
    "session_id": "sess_123"
  }
  ```
- **Example curl**:
  ```sh
  curl -X POST -H "Content-Type: application/json" \
    -d '{"session_id": "sess_123"}' \
    http://127.0.0.1:8080/halt
  ```

### 3. `POST /answer`
Sends answers/replies to an interactive question hook raised by the agent (e.g., when the user needs to provide clarification).

- **Request Body**:
  ```json
  {
    "session_id": "sess_123",
    "trajectory_id": "traj_xyz",
    "step_index": 2,
    "responses": [
      {
        "selected_option_ids": ["opt_1"],
        "freeform_response": "My clarification answer",
        "skipped": false
      }
    ],
    "cancelled": false
  }
  ```
- **Example curl**:
  ```sh
  curl -X POST -H "Content-Type: application/json" \
    -d '{"session_id": "sess_123", "trajectory_id": "traj_xyz", "step_index": 2, "responses": [{"freeform_response": "Use python"}], "cancelled": false}' \
    http://127.0.0.1:8080/answer
  ```

### 4. `POST /confirm`
Approves or denies a pending tool execution confirmation request (e.g., confirming a command execution policy).

- **Request Body**:
  ```json
  {
    "session_id": "sess_123",
    "trajectory_id": "traj_xyz",
    "step_index": 3,
    "accepted": true,
    "allow_for_session": false,
    "tool_name": "RUN_COMMAND"
  }
  ```
- **Example curl**:
  ```sh
  curl -X POST -H "Content-Type: application/json" \
    -d '{"session_id": "sess_123", "trajectory_id": "traj_xyz", "step_index": 3, "accepted": true}' \
    http://127.0.0.1:8080/confirm
  ```

### 5. `GET /workspace`
Gets the workspace path configured for a session.

- **Query Parameters**:
  - `session_id` (String)
- **Example curl**:
  ```sh
  curl "http://127.0.0.1:8080/workspace?session_id=sess_123"
  ```

### 6. `POST /workspace`
Sets or overrides the workspace directory path for a session.

- **Request Body**:
  ```json
  {
    "session_id": "sess_123",
    "path": "/absolute/path/to/workspace"
  }
  ```

### 7. `GET /resolve/folder`
Resolves details (e.g., metadata, file contents) for a workspace folder.

- **Query Parameters**:
  - `session_id` (String)
- **Example curl**:
  ```sh
  curl "http://127.0.0.1:8080/resolve/folder?session_id=sess_123"
  ```

### 8. `GET /health`
Verifies that the server is online.

- **Example curl**:
  ```sh
  curl http://127.0.0.1:8080/health
  ```
