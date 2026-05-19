use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

pub const DEFAULT_MODEL: &str = "gemini-3.5-flash";
pub const DEFAULT_IMAGE_GENERATION_MODEL: &str = "gemini-3.1-flash-image-preview";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ThinkingLevel {
    Minimal,
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_level: Option<ThinkingLevel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelEntry {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    #[serde(default)]
    pub generation: GenerationConfig,
}

impl Default for ModelEntry {
    fn default() -> Self {
        Self {
            name: DEFAULT_MODEL.to_string(),
            api_key: None,
            generation: GenerationConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    #[serde(default = "default_model_entry")]
    pub default: ModelEntry,
    #[serde(default = "default_image_generation_entry")]
    pub image_generation: ModelEntry,
}

fn default_model_entry() -> ModelEntry {
    ModelEntry {
        name: DEFAULT_MODEL.to_string(),
        api_key: None,
        generation: GenerationConfig::default(),
    }
}

fn default_image_generation_entry() -> ModelEntry {
    ModelEntry {
        name: DEFAULT_IMAGE_GENERATION_MODEL.to_string(),
        api_key: None,
        generation: GenerationConfig::default(),
    }
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            default: default_model_entry(),
            image_generation: default_image_generation_entry(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GeminiConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    #[serde(default)]
    pub models: ModelConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInstructionSection {
    pub content: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomSystemInstructions {
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppendedSystemInstructions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_identity: Option<String>,
    #[serde(default)]
    pub appended_sections: Vec<SystemInstructionSection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SystemInstructions {
    Custom(CustomSystemInstructions),
    Appended(AppendedSystemInstructions),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum BuiltinTools {
    #[serde(rename = "CREATE_FILE")]
    CreateFile,
    #[serde(rename = "EDIT_FILE")]
    EditFile,
    #[serde(rename = "FIND_FILE")]
    FindFile,
    #[serde(rename = "LIST_DIR")]
    ListDir,
    #[serde(rename = "RUN_COMMAND")]
    RunCommand,
    #[serde(rename = "SEARCH_DIR")]
    SearchDir,
    #[serde(rename = "VIEW_FILE")]
    ViewFile,
    #[serde(rename = "START_SUBAGENT")]
    StartSubagent,
    #[serde(rename = "GENERATE_IMAGE")]
    GenerateImage,
    #[serde(rename = "FINISH")]
    Finish,
}

impl BuiltinTools {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::CreateFile => "CREATE_FILE",
            Self::EditFile => "EDIT_FILE",
            Self::FindFile => "FIND_FILE",
            Self::ListDir => "LIST_DIR",
            Self::RunCommand => "RUN_COMMAND",
            Self::SearchDir => "SEARCH_DIR",
            Self::ViewFile => "VIEW_FILE",
            Self::StartSubagent => "START_SUBAGENT",
            Self::GenerateImage => "GENERATE_IMAGE",
            Self::Finish => "FINISH",
        }
    }

    pub fn read_only() -> Vec<Self> {
        vec![
            Self::FindFile,
            Self::ListDir,
            Self::ViewFile,
            Self::SearchDir,
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CapabilitiesConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled_tools: Option<Vec<BuiltinTools>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled_tools: Option<Vec<BuiltinTools>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compaction_threshold: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_tool_schema_json: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum McpServerConfig {
    #[serde(rename = "stdio")]
    Stdio { command: String, args: Vec<String> },
    #[serde(rename = "sse")]
    Sse {
        url: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        headers: Option<HashMap<String, String>>,
    },
    #[serde(rename = "http")]
    Http {
        url: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        headers: Option<HashMap<String, String>>,
        #[serde(default = "default_mcp_timeout")]
        timeout: f64,
        #[serde(default = "default_mcp_sse_timeout")]
        sse_read_timeout: f64,
        #[serde(default = "default_true")]
        terminate_on_close: bool,
    },
}

const fn default_mcp_timeout() -> f64 {
    30.0
}
const fn default_mcp_sse_timeout() -> f64 {
    300.0
}
const fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub args: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub canonical_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UsageMetadata {
    pub prompt_token_count: i32,
    pub candidates_token_count: i32,
    pub total_token_count: i32,
    #[serde(default)]
    pub cached_content_token_count: i32,
    #[serde(default)]
    pub thoughts_token_count: i32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum StepType {
    #[serde(rename = "TEXT_RESPONSE")]
    TextResponse,
    #[serde(rename = "TOOL_CALL")]
    ToolCall,
    #[serde(rename = "SYSTEM_MESSAGE")]
    SystemMessage,
    #[serde(rename = "COMPACTION")]
    Compaction,
    #[serde(rename = "FINISH")]
    Finish,
    #[serde(rename = "UNKNOWN")]
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum StepSource {
    #[serde(rename = "SYSTEM")]
    System,
    #[serde(rename = "USER")]
    User,
    #[serde(rename = "MODEL")]
    Model,
    #[serde(rename = "UNKNOWN")]
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum StepTarget {
    #[serde(rename = "TARGET_USER")]
    User,
    #[serde(rename = "TARGET_ENVIRONMENT")]
    Environment,
    #[serde(rename = "TARGET_UNSPECIFIED")]
    Unspecified,
    #[serde(rename = "UNKNOWN")]
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum StepStatus {
    #[serde(rename = "ACTIVE")]
    Active,
    #[serde(rename = "DONE")]
    Done,
    #[serde(rename = "WAITING_FOR_USER")]
    WaitingForUser,
    #[serde(rename = "ERROR")]
    Error,
    #[serde(rename = "CANCELED")]
    Canceled,
    #[serde(rename = "UNKNOWN")]
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step {
    pub id: String,
    pub step_index: u32,
    pub r#type: StepType,
    pub source: StepSource,
    pub target: StepTarget,
    pub status: StepStatus,
    pub content: String,
    pub content_delta: String,
    pub thinking: String,
    pub thinking_delta: String,
    pub tool_calls: Vec<ToolCall>,
    pub error: String,
    pub is_complete_response: Option<bool>,
    pub structured_output: Option<Value>,
    pub usage_metadata: Option<UsageMetadata>,
    // LocalConnectionStep specific fields
    #[serde(default)]
    pub cascade_id: String,
    #[serde(default)]
    pub trajectory_id: String,
    #[serde(default)]
    pub http_code: u32,
}

impl Default for Step {
    fn default() -> Self {
        Self {
            id: String::new(),
            step_index: 0,
            r#type: StepType::Unknown,
            source: StepSource::Unknown,
            target: StepTarget::Unknown,
            status: StepStatus::Unknown,
            content: String::new(),
            content_delta: String::new(),
            thinking: String::new(),
            thinking_delta: String::new(),
            tool_calls: Vec::new(),
            error: String::new(),
            is_complete_response: None,
            structured_output: None,
            usage_metadata: None,
            cascade_id: String::new(),
            trajectory_id: String::new(),
            http_code: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HookResult {
    pub allow: bool,
    #[serde(default)]
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionResponse {
    pub selected_option_ids: Option<Vec<String>>,
    #[serde(default)]
    pub freeform_response: String,
    #[serde(default)]
    pub skipped: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionHookResult {
    pub responses: Vec<QuestionResponse>,
    #[serde(default)]
    pub cancelled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AskQuestionOption {
    pub id: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AskQuestionEntry {
    pub question: String,
    pub options: Vec<AskQuestionOption>,
    #[serde(default)]
    pub is_multi_select: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub text: String,
    pub thinking: String,
    pub steps: Vec<Step>,
    pub usage_metadata: UsageMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "chunk_type")]
pub enum StreamChunk {
    Thought { step_index: u32, text: String },
    Text { step_index: u32, text: String },
    ToolCall(ToolCall),
}
