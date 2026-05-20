//! Asynchronous event triggers for background connection orchestration.
//!
//! This module defines the [`Trigger`] trait, permitting autonomous tasks or background workers
//! (e.g. status polling, cron intervals, external notification listeners) to interact with the
//! connection session asynchronously. Background orchestration of these tasks is handled via [`TriggerRunner`].

use futures_util::future::BoxFuture;
use std::sync::Arc;

/// A trait for defining asynchronous background tasks that execute during a connection lifecycle.
pub trait Trigger: Send + Sync {
    /// Launches the trigger task, passing the active [`AnyConnection`](crate::connection::AnyConnection) instance.
    ///
    /// # Errors
    ///
    /// Returns an error if the background execution encounters a fatal issue.
    fn run(
        &self,
        connection: crate::connection::AnyConnection,
    ) -> impl std::future::Future<Output = Result<(), anyhow::Error>> + Send;
}

/// Object-safe version of the [`Trigger`] trait, automatically implemented via a blanket impl.
///
/// This trait is used internally by the SDK to allow dynamic dispatch and storage of triggers.
pub trait DynTrigger: Send + Sync {
    /// Launches the trigger task.
    fn run(
        &self,
        connection: crate::connection::AnyConnection,
    ) -> BoxFuture<'_, Result<(), anyhow::Error>>;
}

impl<T: Trigger + ?Sized> DynTrigger for T {
    fn run(
        &self,
        connection: crate::connection::AnyConnection,
    ) -> BoxFuture<'_, Result<(), anyhow::Error>> {
        Box::pin(async move { self.run(connection).await })
    }
}

/// Orchestrator for launching background [`Trigger`] loops.
pub struct TriggerRunner {
    /// Registered trigger instances.
    pub triggers: Vec<Arc<dyn DynTrigger>>,
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
    pub fn new(triggers: Vec<Arc<dyn DynTrigger>>) -> Self {
        Self { triggers }
    }

    /// Spawns each registered trigger inside a new asynchronous tokio task block.
    pub fn start(&self, connection: &crate::connection::AnyConnection) {
        for trigger in &self.triggers {
            let conn = connection.clone();
            let tr = trigger.clone();
            crate::spawn_task(async move {
                if let Err(e) = tr.run(conn).await {
                    tracing::error!("Trigger execution failed: {:?}", e);
                }
            });
        }
    }
}
