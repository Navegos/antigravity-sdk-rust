use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters_json_schema(&self) -> &str;
    async fn call(&self, args: Value) -> Result<Value, anyhow::Error>;
}

#[derive(Clone, Default)]
pub struct ToolRunner {
    pub tools: Arc<tokio::sync::RwLock<HashMap<String, Arc<dyn Tool>>>>,
}

impl std::fmt::Debug for ToolRunner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolRunner")
            .field("tools_count", &self.tools.try_read().map_or(0, |t| t.len()))
            .finish()
    }
}

impl ToolRunner {
    pub fn new() -> Self {
        Self {
            tools: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    pub async fn register(&self, tool: Arc<dyn Tool>) {
        self.tools
            .write()
            .await
            .insert(tool.name().to_string(), tool);
    }

    pub async fn process_tool_calls(
        &self,
        calls: Vec<crate::types::ToolCall>,
    ) -> Vec<crate::types::ToolResult> {
        let mut results = Vec::new();
        for call in calls {
            let tools = self.tools.read().await;
            if let Some(tool) = tools.get(&call.name) {
                match tool.call(call.args).await {
                    Ok(val) => {
                        results.push(crate::types::ToolResult {
                            id: Some(call.id),
                            name: call.name.clone(),
                            result: Some(val),
                            error: None,
                        });
                    }
                    Err(e) => {
                        results.push(crate::types::ToolResult {
                            id: Some(call.id),
                            name: call.name.clone(),
                            result: None,
                            error: Some(e.to_string()),
                        });
                    }
                }
            } else {
                results.push(crate::types::ToolResult {
                    id: Some(call.id),
                    name: call.name.clone(),
                    result: None,
                    error: Some(format!("Tool {} not found", call.name)),
                });
            }
        }
        results
    }
}
