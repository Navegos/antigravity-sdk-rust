# Built-In Tools & Capabilities

The Google Antigravity Rust SDK provides access to a set of native system capabilities powered by the `localharness` binary.

## Available Built-In Tools

| Tool Enum Name | Command / Protocol name | Description | Read-Only |
|---|---|---|---|
| `BuiltinTools::CreateFile` | `CREATE_FILE` | Creates a new file at a specific path with code contents. | No |
| `BuiltinTools::EditFile` | `EDIT_FILE` | Modifies existing file segments based on line offsets. | No |
| `BuiltinTools::FindFile` | `FIND_FILE` | Scans for filenames matches in the workspace. | Yes |
| `BuiltinTools::ListDir` | `LIST_DIR` | Lists the immediate children of a directory. | Yes |
| `BuiltinTools::RunCommand`| `RUN_COMMAND` | Executes shell commands in the workspace environment. | No |
| `BuiltinTools::SearchDir` | `SEARCH_DIR` | Searches text content within files matching patterns. | Yes |
| `BuiltinTools::ViewFile` | `VIEW_FILE` | Views the contents of text or supported binary files. | Yes |
| `BuiltinTools::StartSubagent`| `START_SUBAGENT` | Spawns a subagent to delegate tasks. | No |
| `BuiltinTools::GenerateImage`| `GENERATE_IMAGE` | Generates or edits visual media content. | No |
| `BuiltinTools::Finish` | `FINISH` | Completes the conversation trajectory. | No |

---

## Configuring Enabled Capabilities

By default, all built-in tools are enabled. You can restrict the agent's tool access by setting the `CapabilitiesConfig` struct inside `AgentConfig`.

> [!WARNING]
> You can specify `enabled_tools` OR `disabled_tools`, but not both. They are mutually exclusive.

### Example: Denying Shell Commands (Read-Only Agent)

```rust
use antigravity_sdk_rust::agent::{Agent, AgentConfig};
use antigravity_sdk_rust::types::{CapabilitiesConfig, BuiltinTools};

let mut config = AgentConfig::default();

// Only enable file reading and search tools
config.capabilities = CapabilitiesConfig {
    enabled_tools: Some(vec![
        BuiltinTools::ListDir,
        BuiltinTools::ViewFile,
        BuiltinTools::SearchDir,
        BuiltinTools::FindFile,
        BuiltinTools::Finish,
    ]),
    ..Default::default()
};
```

### Example: Disabling Image Generation

```rust
config.capabilities = CapabilitiesConfig {
    disabled_tools: Some(vec![
        BuiltinTools::GenerateImage,
    ]),
    ..Default::default()
};
```

---

## Advanced Capabilities Settings

The `CapabilitiesConfig` struct also supports:

* **`compaction_threshold`**: Configures the context length at which conversation message history compaction triggers to reduce token usage.
* **`image_model`**: Specifies the model used for image generation tasks (e.g. Imagen models).

---

## Tool Details: RUN_COMMAND

The `RUN_COMMAND` tool allows the agent to execute shell commands in the workspace environment.

### Environment Variable Forwarding
To ensure that build tools, compilers, and CLI runtimes are correctly found, `RUN_COMMAND` automatically inherits and forwards critical environment variables from the host process:
* **`PATH`**: Used to locate executable binaries.
* **`SHELL`**: Specifies the user's default shell for execution.

### Execution Output & Exit Code
When `RUN_COMMAND` runs a command, it captures both stdout and stderr in a merged output stream, returning the following fields in the tool call and output payload:
* **`combined_output`**: A string containing the merged stdout and stderr from the command.
* **`exit_code`**: An integer indicating the process exit code (e.g., `0` for success).
