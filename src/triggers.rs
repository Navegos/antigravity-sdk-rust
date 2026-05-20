//! Asynchronous event triggers for background connection orchestration.
//!
//! This module defines the [`Trigger`] trait, permitting autonomous tasks or background workers
//! (e.g. status polling, cron intervals, external notification listeners) to interact with the
//! connection session asynchronously. Background orchestration of these tasks is handled via [`TriggerRunner`].

use async_trait::async_trait;
use std::sync::Arc;

/// A trait for defining asynchronous background tasks that execute during a connection lifecycle.
#[async_trait]
pub trait Trigger: Send + Sync {
    /// Launches the trigger task, passing the active [`Connection`](crate::connection::Connection) instance.
    ///
    /// # Errors
    ///
    /// Returns an error if the background execution encounters a fatal issue.
    async fn run(
        &self,
        connection: Arc<dyn crate::connection::Connection>,
    ) -> Result<(), anyhow::Error>;
}

/// Orchestrator for launching background [`Trigger`] loops.
pub struct TriggerRunner {
    /// Registered trigger instances.
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
    /// Creates a new `TriggerRunner` initialized with the given list of triggers.
    pub fn new(triggers: Vec<Arc<dyn Trigger>>) -> Self {
        Self { triggers }
    }

    /// Spawns each registered trigger inside a new asynchronous tokio task block.
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
