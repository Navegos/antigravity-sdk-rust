# Antigravity Rust SDK Architecture

This document describes the high-level architecture, design patterns, and components of the Antigravity Rust SDK.

---

## High-Level Overview

The Antigravity SDK orchestrates interactions between an LLM-based agent (running inside a local or remote harness) and the local client system. It manages process execution, IPC handshake, WebSocket event streaming, tool calls, policy middleware, and user hooks.

```mermaid
graph TD
    A[Agent] --> B[Conversation]
    A --> C[LocalConnection / WasmConnection]
    A --> D[ToolRunner]
    A --> E[HookRunner]
    A --> I[TriggerRunner]
    C -->|Subprocess IPC / WebSocket| F[localharness]
    C -->|Network WebSocket| F
    D -->|Executes| G[Local Tools]
    E -->|Intercepts| H[User Hooks / Policies]
    I -->|Spawns| J[Background Triggers]
```

---

## Design Patterns

The SDK leverages several object-oriented and functional design patterns:

### 1. Connection & Strategy Pattern (`connection.rs`, `local.rs`, `wasm.rs`)
- **Connection Trait**: Defines an abstraction for communication. This allows swapping the local subprocess harness with other backends (e.g., remote, mock, or WASM-based network harnesses) in the future.
- **LocalConnectionStrategy**: Configures and initializes a `LocalConnection` by spawning a local helper subprocess (for native / non-WASM environments).
- **WasmConnectionStrategy**: Configures and initializes a `WasmConnection` that connects to a remote or host-side `localharness` WebSocket server over TCP (enabled for `target_arch = "wasm32"` environments where subprocess spawning is not supported).

### 2. Observer Pattern (`hooks.rs`)
- **Hook Trait**: Defines lifecycle hooks that users can register to observe and modify agent actions:
  - `on_session_start()`
  - `pre_turn()`
  - `pre_tool_call()`
  - `post_tool_call()`
  - `on_tool_error()`
  - `on_interaction()`
- **HookRunner**: Coordinates a thread-safe list of observers (`Arc<dyn Hook>`) and dispatches events asynchronously.

### 3. Middleware / Interceptor Pattern (`policy.rs`)
- **Policy**: Acts as a middleware layer to authorize, deny, or intercept tool calls before they are executed.
- Included Policies:
  - `workspace_only(paths)`: Blocks tool calls targeting directories outside the specified workspaces.
  - `confirm_run_command()`: Prompts user authorization or automatically enforces constraints before running shell commands.

### 4. Command Pattern (`tools.rs`)
- **Tool Trait**: Encapsulates specific capabilities (e.g., file edits, command execution, directory searching) into unified command units.
- **ToolRunner**: Coordinates registration and execution of these command objects, mapping harness tool calls to their respective handlers.

### 5. Background Trigger Pattern (`triggers.rs`)
- **Trigger Trait**: Defines asynchronous background tasks (such as status polling, listener intervals, etc.) that can interact with the connection session concurrently.
- **TriggerRunner**: Coordinates and spawns registered triggers in separate tasks when the agent session starts.
---

## Component Details

### Connection Lifecycle (Native Subprocess)

The connection to `localharness` via subprocess follows a strict handshake and upgrade protocol:

```mermaid
sequenceDiagram
    participant SDK as Rust SDK
    participant Sub as Subprocess (localharness)
    participant WS as WebSocket Server

    SDK->>Sub: Spawn subprocess (stdout/stderr piped)
    SDK->>Sub: Send InputConfig (length-prefixed proto)
    Sub->>SDK: Reply OutputConfig (length-prefixed proto with Port & API Key)
    Note over SDK,Sub: Stdin/Stdout handshake complete
    
    SDK->>WS: Establish WebSocket Connection (with API Key header)
    WS->>SDK: Handshake Completed
    
    Note over SDK,WS: Step execution loop active
```

1. **Subprocess Spawn**: The SDK spawns the `localharness` binary as a child process.
2. **Handshake**: The SDK sends an `InputConfig` (serialized protocol buffer, prefixed by its length in bytes) over stdin. The harness replies with an `OutputConfig` containing the dynamically selected port and a secure API key.
3. **Upgrade**: The SDK initiates a WebSocket client connection to the harness server using the retrieved port and API key, upgrading communication to a structured bi-directional stream.
4. **Disconnection**: When dropped, the subprocess is killed cleanly.

### Connection Lifecycle (WebAssembly Network Target)

For WebAssembly targets (`wasm32-wasip1`), the SDK connects to a running host-side harness over the network rather than spawning a subprocess:

```mermaid
sequenceDiagram
    participant SDK as WASM Rust SDK
    participant Host as Host Machine (localharness)

    SDK->>Host: Open TCP Connection & Upgrade to WebSocket (with API Key)
    Host->>SDK: Handshake Completed
    SDK->>Host: Send InitializeConversationEvent (HarnessConfig)
    Note over SDK,Host: Step execution loop active
```

---

## Thread Safety & Concurrency

- **Lock Scoping**: Mutexes (`tokio::sync::Mutex`) are carefully scoped to minimize contention. Mutex guards are explicitly dropped before any `.await` points to avoid deadlocks.
- **Hook Dispatch**: Hook guards are cloned and dropped prior to executing hooks asynchronously, ensuring the agent's internal state remains responsive.
