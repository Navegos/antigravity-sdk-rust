# Safety Policies & Interceptors

The Google Antigravity Rust SDK features a declarative safety policy engine that intercepts and evaluates tool calls before execution.

## The Policy & Decision Engine

Every tool execution request is passed through a sequence of `Policy` middleware components. A `Policy` is evaluated to a `Decision`:

```rust
pub enum Decision {
    Approve,
    Deny,
    AskUser,
}
```

* **`Approve`**: Explicitly permits execution.
* **`Deny`**: Instantly blocks execution and returns a policy rejection message to the LLM.
* **`AskUser`**: Intercepts the call and prompts the user for verification.

---

## Built-In Policies

The SDK provides several built-in policies inside the `policy` module:

### 1. Allow All / Deny All
```rust
use antigravity_sdk_rust::policy;

// Danger: allows the agent to execute any tool (including commands/writes) without constraint
let policy1 = policy::allow_all();

// Secure default: block all tools unless explicitly allowed
let policy2 = policy::deny_all();
```

### 2. Workspace Scoping
Restricts all filesystem tool calls to specific target folders:
```rust
let allowed_directories = vec!["/Volumes/data/sandbox".to_string()];
let ws_policy = policy::workspace_only(allowed_directories);
```

### 3. Command Confirmation
Prompts the user for approval before running shell commands:
```rust
// Requires user prompt for any RUN_COMMAND tool calls
let cmd_policy = policy::confirm_run_command(None);
```

---

## Ordering and Rules Evaluation

> [!IMPORTANT]
> Policies are compiled into a **6-bucket priority system** where specificity and safety determine precedence:
>
> `Specific Deny > Specific Ask > Specific Approve > Wildcard Deny > Wildcard Ask > Wildcard Approve`
>
> Within each priority group, policies are evaluated in registration order (first match wins). This means a `deny("tool_name")` always takes priority over an `allow("tool_name")`, and any specific-tool rule always takes priority over a wildcard (`*`) rule, regardless of insertion order.

When custom workspaces are defined in `AgentConfig::workspaces`, workspace scoping policies (e.g., `policy::workspace_only(...)`) are automatically prepended, ensuring filesystem path locks are evaluated first.

> [!NOTE]
> **Bypassing the Workspace Gate**: If the policy configuration contains `policy::allow_all()`, the SDK detects it and **skips prepending workspace scoping policies entirely**. This allows developers who use external approval workflows (like standard `ConfirmHook` handlers) to manage directory validation without the default SDK workspace locks overriding their approvals.

---

## Custom Policies & Predicates

You can declare custom policies by defining target tool names, decisions, and predicate functions (`Fn(&ToolCall) -> bool`):

```rust
use antigravity_sdk_rust::policy::{Policy, Decision};
use antigravity_sdk_rust::types::ToolCall;
use std::sync::Arc;

// Predicate that matches command lines containing 'rm'
fn is_delete_command(tool_call: &ToolCall) -> bool {
    tool_call.args
        .get("CommandLine")
        .and_then(|v| v.as_str())
        .is_some_and(|cmd| cmd.contains("rm"))
}

// Custom Policy configuration
let block_rm = Policy::new(
    "RUN_COMMAND".to_string(),
    Decision::Deny,
    Some(Arc::new(is_delete_command)),
    None,
    "block-rm-commands".to_string(),
);
```

### Programmatic Ask-User Approvals
For `Decision::AskUser`, you can supply a custom handler function to determine if the task should proceed programmatically:

```rust
fn custom_approval_handler(tool_call: &ToolCall) -> bool {
    println!("Checking if tool call is authorized: {:?}", tool_call);
    // Return true to allow, false to deny
    false
}

let ask_user_policy = Policy::new(
    "CREATE_FILE".to_string(),
    Decision::AskUser,
    None, // Matches all file writes
    Some(Arc::new(custom_approval_handler)),
    "ask-writes".to_string(),
);
```
# Pre-Turn and Tool Hooks
In addition to the policy middleware engine, you can register general lifecycle callbacks using the `Hook` trait to audit or change variables, described in [error_handling.md](file:///Volumes/goldcoders/antigravity-sdk-rust/skills/google-antigravity-sdk-rust/references/error_handling.md).
