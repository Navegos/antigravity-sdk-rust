use crate::conversation::Conversation;
use crate::hooks::{Hook, HookRunner};
use crate::local::LocalConnectionStrategy;
use crate::policy::{self, Policy, PolicyEnforcer};
use crate::tools::{Tool, ToolRunner};
use crate::triggers::{Trigger, TriggerRunner};
use crate::types::{
    BuiltinTools, CapabilitiesConfig, ChatResponse, GeminiConfig, SystemInstructions,
};
use anyhow::anyhow;
use std::sync::Arc;

#[derive(Default)]
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

impl std::fmt::Debug for AgentConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AgentConfig")
            .field("binary_path", &self.binary_path)
            .field("gemini_config", &self.gemini_config)
            .field("capabilities", &self.capabilities)
            .field("system_instructions", &self.system_instructions)
            .field("save_dir", &self.save_dir)
            .field("workspaces", &self.workspaces)
            .field("skills_paths", &self.skills_paths)
            .field("policies", &self.policies)
            .field("hooks_count", &self.hooks.len())
            .field("triggers_count", &self.triggers.len())
            .field("tools_count", &self.tools.len())
            .field("conversation_id", &self.conversation_id)
            .field("app_data_dir", &self.app_data_dir)
            .field("response_schema", &self.response_schema)
            .finish()
    }
}

pub struct Agent {
    config: AgentConfig,
    conversation: Option<Arc<Conversation>>,
    tool_runner: ToolRunner,
    hook_runner: HookRunner,
    trigger_runner: Option<TriggerRunner>,
}

impl std::fmt::Debug for Agent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Agent")
            .field("config", &self.config)
            .field("conversation", &self.conversation)
            .field("tool_runner", &self.tool_runner)
            .field("hook_runner", &self.hook_runner)
            .field("trigger_runner", &self.trigger_runner)
            .finish()
    }
}

impl Agent {
    pub fn new(config: AgentConfig) -> Self {
        Self {
            config,
            conversation: None,
            tool_runner: ToolRunner::new(),
            hook_runner: HookRunner::new(),
            trigger_runner: None,
        }
    }

    pub fn register_hook(&self, hook: Arc<dyn Hook>) {
        let hr = self.hook_runner.clone();
        tokio::spawn(async move {
            hr.register(hook).await;
        });
    }

    pub fn register_trigger(&mut self, trigger: Arc<dyn Trigger>) -> Result<(), anyhow::Error> {
        if self.conversation.is_some() {
            return Err(anyhow!(
                "Cannot register triggers after the agent has started."
            ));
        }
        self.config.triggers.push(trigger);
        Ok(())
    }

    pub fn register_tool(&self, tool: Arc<dyn Tool>) {
        let tr = self.tool_runner.clone();
        tokio::spawn(async move {
            tr.register(tool).await;
        });
    }

    #[allow(clippy::too_many_lines)]
    pub async fn start(&mut self) -> Result<(), anyhow::Error> {
        if self.conversation.is_some() {
            return Ok(());
        }

        // 1. Resolve binary path
        let binary_path = self.config.binary_path.clone()
            .or_else(get_default_binary_path)
            .ok_or_else(|| anyhow!("Could not find default localharness binary. Please specify binary_path explicitly."))?;

        // 2. Setup hook runner and register pending hooks
        for hook in &self.config.hooks {
            self.hook_runner.register(hook.clone()).await;
        }

        // 3. Process capabilities and active tools
        let enabled_tools = self.config.capabilities.enabled_tools.clone();
        let disabled_tools = self.config.capabilities.disabled_tools.clone();
        if enabled_tools.is_some() && disabled_tools.is_some() {
            return Err(anyhow!(
                "enabled_tools and disabled_tools are mutually exclusive"
            ));
        }

        let active_tools = enabled_tools.unwrap_or_else(|| {
            disabled_tools.map_or_else(
                || {
                    vec![
                        BuiltinTools::CreateFile,
                        BuiltinTools::EditFile,
                        BuiltinTools::FindFile,
                        BuiltinTools::ListDir,
                        BuiltinTools::RunCommand,
                        BuiltinTools::SearchDir,
                        BuiltinTools::ViewFile,
                        BuiltinTools::StartSubagent,
                        BuiltinTools::GenerateImage,
                        BuiltinTools::Finish,
                    ]
                },
                |disabled| {
                    let all = vec![
                        BuiltinTools::CreateFile,
                        BuiltinTools::EditFile,
                        BuiltinTools::FindFile,
                        BuiltinTools::ListDir,
                        BuiltinTools::RunCommand,
                        BuiltinTools::SearchDir,
                        BuiltinTools::ViewFile,
                        BuiltinTools::StartSubagent,
                        BuiltinTools::GenerateImage,
                        BuiltinTools::Finish,
                    ];
                    all.into_iter().filter(|t| !disabled.contains(t)).collect()
                },
            )
        });

        let read_only = BuiltinTools::read_only();
        let has_write_tools = active_tools.iter().any(|t| !read_only.contains(t));

        // 4. Set up policies
        let mut final_policies = self.config.policies.clone().unwrap_or_else(|| {
            // Default to confirm_run_command
            policy::confirm_run_command(None)
        });

        // Prepend workspace scoping policies if workspaces are configured
        let workspaces = self.config.workspaces.clone().unwrap_or_else(|| {
            std::env::current_dir().map_or_else(
                |_| Vec::new(),
                |cwd| vec![cwd.to_string_lossy().into_owned()],
            )
        });

        if !workspaces.is_empty() {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
            let app_data_dir = self
                .config
                .app_data_dir
                .clone()
                .unwrap_or_else(|| format!("{home}/.gemini/antigravity"));
            let mut allowed_paths = workspaces;
            allowed_paths.push(app_data_dir);
            let mut ws_policies = policy::workspace_only(allowed_paths);
            ws_policies.append(&mut final_policies);
            final_policies = ws_policies;
        }

        // Safety policy check: if write tools are enabled, policies cannot be empty
        if has_write_tools && final_policies.is_empty() {
            return Err(anyhow!(
                "Write tools are enabled without a safety policy. Add policies=[policy.allow_all()] to approve all tool calls, or policies=[policy.deny_all(), policy.allow(\"tool_name\")] to selectively allow specific tools."
            ));
        }

        if !final_policies.is_empty() {
            let enforcer = Arc::new(PolicyEnforcer::new(final_policies));
            self.hook_runner.register(enforcer).await;
        }

        // 5. Register configured tools
        for tool in &self.config.tools {
            self.tool_runner.register(tool.clone()).await;
        }

        // 6. Build and connect strategy
        let mut cap = self.config.capabilities.clone();
        if let Some(ref schema) = self.config.response_schema {
            cap.finish_tool_schema_json = Some(schema.clone());
        }

        let strategy = LocalConnectionStrategy::new(
            binary_path,
            self.config.gemini_config.clone(),
            cap,
            self.config.system_instructions.clone(),
            self.config.save_dir.clone(),
            self.config.workspaces.clone().unwrap_or_default(),
            self.config.skills_paths.clone(),
            Some(self.tool_runner.clone()),
            Some(self.hook_runner.clone()),
            self.config.conversation_id.clone().unwrap_or_default(),
        );

        let conn = strategy.connect().await?;
        let conversation = Arc::new(Conversation::new(Arc::new(conn), None));
        self.conversation = Some(conversation.clone());

        // 7. Start triggers
        if !self.config.triggers.is_empty() {
            let runner = TriggerRunner::new(self.config.triggers.clone());
            runner.start(&conversation.connection());
            self.trigger_runner = Some(runner);
        }

        Ok(())
    }

    pub async fn chat(&self, prompt: &str) -> Result<ChatResponse, anyhow::Error> {
        let conversation = self.conversation()?;
        conversation.chat_to_completion(prompt).await
    }

    pub fn conversation(&self) -> Result<Arc<Conversation>, anyhow::Error> {
        self.conversation
            .clone()
            .ok_or_else(|| anyhow!("Agent session not started. Use start() first."))
    }

    pub fn conversation_id(&self) -> Option<String> {
        self.conversation
            .as_ref()
            .map(|c| c.conversation_id().to_string())
    }

    pub async fn stop(&mut self) -> Result<(), anyhow::Error> {
        if let Some(conversation) = self.conversation.take() {
            conversation.disconnect().await?;
        }
        Ok(())
    }
}

fn get_default_binary_path() -> Option<String> {
    if let Ok(path) = std::env::var("ANTIGRAVITY_HARNESS_PATH") {
        return Some(path);
    }
    // Check if it is in standard PATH
    if let Ok(paths) = std::env::var("PATH") {
        for path in std::env::split_paths(&paths) {
            let p = path.join("localharness");
            if p.exists() {
                return Some(p.to_string_lossy().into_owned());
            }
        }
    }
    None
}
