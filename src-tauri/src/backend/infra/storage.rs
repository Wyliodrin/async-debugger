//! Defines the `Storage` trait to unify reads/writes of all domain data.

use super::guard::WriteableDataBaseGuard;
use crate::backend::domain::{
    application::Application,
    async_op::{AsyncOp, TaskOp},
    poll::Poll,
    resource::Resource,
    Task,
};
use async_trait::async_trait;
use std::path::PathBuf;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use uuid::Uuid;

/// Abstraction over a persisted store for all domain entities.
/// Implementors must provide both read‐only and write guards.
#[async_trait]
pub(crate) trait Storage: Send + Sync {
    /// Read all applications as an `Arc<Application>` map.
    async fn applications_read(&self) -> HashMap<Uuid, Arc<Application>>;

    /// Acquire a write guard for applications; on drop, writes to disk.
    async fn applications_write(
        &self,
    ) -> WriteableDataBaseGuard<'_, HashMap<Uuid, Arc<Application>>>;

    /// Read all tasks.
    async fn tasks_read(&self) -> HashMap<String, Arc<Task>>;

    /// Write guard for tasks.
    async fn tasks_write(&self) -> WriteableDataBaseGuard<'_, HashMap<String, Arc<Task>>>;

    async fn active_tasks_write(&self) -> tokio::sync::RwLockWriteGuard<'_, HashSet<String>>;

    async fn active_tasks_read(&self) -> HashSet<String>;

    /// Read all resources.
    async fn resources_read(&self) -> HashMap<String, Arc<Resource>>;

    /// Write guard for resources.
    async fn resources_write(&self) -> WriteableDataBaseGuard<'_, HashMap<String, Arc<Resource>>>;

    /// Read all polls.
    async fn polls_read(&self) -> Vec<Arc<Poll>>;

    /// Write guard for polls.
    async fn polls_write(&self) -> WriteableDataBaseGuard<'_, Vec<Arc<Poll>>>;

    /// Read all async operations.
    async fn async_ops_read(&self) -> HashMap<String, Arc<AsyncOp>>;

    /// Write guard for async operations.
    async fn async_ops_write(&self) -> WriteableDataBaseGuard<'_, HashMap<String, Arc<AsyncOp>>>;

    /// Read all task operations.
    async fn tasks_ops_read(&self) -> HashMap<String, Arc<TaskOp>>;

    /// Write guard for task operations.
    async fn tasks_ops_write(&self) -> WriteableDataBaseGuard<'_, HashMap<String, Arc<TaskOp>>>;

    async fn import_from_export_folder(&self, folder: PathBuf) -> Result<(), String>;

    async fn export_app_instance(
        &self,
        app_id: String,
        user_name: String,
    ) -> Result<PathBuf, crate::utils::error::Error>;
}
