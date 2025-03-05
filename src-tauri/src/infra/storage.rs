use super::guard::WriteableDataBaseGuard;
use crate::domain::{application::Application, Task};
use async_trait::async_trait;
use std::sync::Arc;

#[async_trait]
pub(crate) trait Storage: Send + Sync {
    async fn applications_read(&self) -> Vec<Arc<Application>>;

    async fn applications_write(&self) -> WriteableDataBaseGuard<'_, Vec<Arc<Application>>>;

    async fn tasks_read(&self) -> Vec<Arc<Task>>;

    async fn tasks_write(&self) -> WriteableDataBaseGuard<'_, Vec<Arc<Task>>>;
}
