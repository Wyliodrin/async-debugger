use super::guard::WriteableDataBaseGuard;
use crate::domain::{application::Application, poll::Poll, resource::Resource, Task};
use async_trait::async_trait;
use std::{collections::HashMap, sync::Arc};
use uuid::Uuid;

#[async_trait]
pub(crate) trait Storage: Send + Sync {
    async fn applications_read(&self) -> HashMap<Uuid, Arc<Application>>;

    async fn applications_write(
        &self,
    ) -> WriteableDataBaseGuard<'_, HashMap<Uuid, Arc<Application>>>;

    async fn tasks_read(&self) -> HashMap<String, Arc<Task>>;

    async fn tasks_write(&self) -> WriteableDataBaseGuard<'_, HashMap<String, Arc<Task>>>;

    async fn resources_read(&self) -> HashMap<String, Arc<Resource>>;

    async fn resources_write(&self) -> WriteableDataBaseGuard<'_, HashMap<String, Arc<Resource>>>;

    async fn polls_read(&self) -> Vec<Arc<Poll>>;

    async fn polls_write(&self) -> WriteableDataBaseGuard<'_, Vec<Arc<Poll>>>;
}
