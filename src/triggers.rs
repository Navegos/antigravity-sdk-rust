use async_trait::async_trait;
use std::sync::Arc;

#[async_trait]
pub trait Trigger: Send + Sync {
    async fn run(
        &self,
        connection: Arc<dyn crate::connection::Connection>,
    ) -> Result<(), anyhow::Error>;
}

pub struct TriggerRunner {
    pub triggers: Vec<Arc<dyn Trigger>>,
}

impl std::fmt::Debug for TriggerRunner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TriggerRunner")
            .field("triggers_count", &self.triggers.len())
            .finish()
    }
}

impl TriggerRunner {
    pub fn new(triggers: Vec<Arc<dyn Trigger>>) -> Self {
        Self { triggers }
    }

    pub fn start(&self, connection: &Arc<dyn crate::connection::Connection>) {
        for trigger in &self.triggers {
            let conn = connection.clone();
            let tr = trigger.clone();
            tokio::spawn(async move {
                if let Err(e) = tr.run(conn).await {
                    tracing::error!("Trigger execution failed: {:?}", e);
                }
            });
        }
    }
}
