# Agent Configuration in Rust

The `AgentConfig` struct manages agent parameters, model choices, workspaces, hooks, and safety policies.

## AgentConfig API Definition

The configuration fields available in `AgentConfig` are:

```rust
pub struct AgentConfig {
    pub binary_path: Option<String>,
    pub gemini_config: GeminiConfig,
    pub capabilities: CapabilitiesConfig,
    pub system_instructions: Option<SystemInstructions>,
    pub save_dir: Option<String>,
    pub workspaces: Option<Vec<String>>,
    pub skills_paths: Vec<String>,
    pub policies: Option<Vec<Policy>>,
    pub hooks: Vec<Arc<dyn Hook>>,
    pub triggers: Vec<Arc<dyn Trigger>>,
    pub tools: Vec<Arc<dyn Tool>>,
    pub conversation_id: Option<String>,
    pub app_data_dir: Option<String>,
    pub response_schema: Option<String>,
}
```

---

## Configuration Categories

### 1. Gemini Configuration (`GeminiConfig`)
Specifies the LLM parameters. 

```rust
use antigravity_sdk_rust::types::{GeminiConfig, ThinkingLevel};

let mut gemini_config = GeminiConfig::default();
gemini_config.api_key = Some("YOUR_API_KEY".to_string());
gemini_config.models.default.name = "gemini-3.5-flash".to_string();

// Optionally configure thinking parameters for reasoning models
gemini_config.models.default.generation.thinking_level = Some(ThinkingLevel::High);
```

### 2. System Instructions
Supports fully custom prompts or modular instructions appended to the system identity:

```rust
use antigravity_sdk_rust::types::{SystemInstructions, CustomSystemInstructions, AppendedSystemInstructions, Section};

// Choice A: Full custom text
let instructions = SystemInstructions::Custom(CustomSystemInstructions {
    text: "You are a helpful software engineering assistant.".to_string(),
});

// Choice B: Sectioned/Appended system instructions
let instructions = SystemInstructions::Appended(AppendedSystemInstructions {
    custom_identity: Some("You are an SRE bot.".to_string()),
    appended_sections: vec![
        Section {
            title: Some("Formatting Guideline".to_string()),
            content: Some("Always wrap commands in code blocks.".to_string()),
        }
    ],
});
```

### 3. Workspaces & Paths
Locks filesystem actions to specific paths:

```rust
// Filesystem tools will be blocked from accessing directories outside these targets
config.workspaces = Some(vec![
    "/path/to/my/project".to_string(),
]);
```

### 4. Skills Paths
Adds paths containing custom Antigravity skill folders containing `SKILL.md` files:

```rust
config.skills_paths = vec![
    "/path/to/shared/skills".to_string(),
];
```

### 5. Response Schema (Structured Output)
Forces the agent's final answer to match a specific JSON Schema:

```rust
let schema = r#"{
    "type": "object",
    "properties": {
        "is_successful": { "type": "boolean" },
        "error_message": { "type": "string" }
    },
    "required": ["is_successful", "error_message"]
}"#;
config.response_schema = Some(schema.to_string());
```
