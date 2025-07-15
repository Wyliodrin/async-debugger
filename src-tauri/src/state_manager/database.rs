use crate::{
    domain::{
        application::Application,
        async_op::{AsyncOp, TaskOp},
        poll::Poll,
        resource::Resource,
        storable::Storable,
        Task,
    },
    error::Error as TraceError,
    infra::{guard::WriteableDataBaseGuard, storage::Storage},
};
use async_trait::async_trait;
use log::{debug, error};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;
use uuid::Uuid;

/// Representation of all data stored on the disk for persistency
/// Provides read/write mechanisms that assure syncronisation with
/// disk files
#[derive(Default)]
pub(crate) struct Database {
    storage_folder: String,
    //applications list
    applications: tokio::sync::RwLock<HashMap<Uuid, Arc<Application>>>,
    //tasks list
    tasks: tokio::sync::RwLock<HashMap<String, Arc<Task>>>,
    //resources list
    resources: tokio::sync::RwLock<HashMap<String, Arc<Resource>>>,
    //polls list
    polls: tokio::sync::RwLock<Vec<Arc<Poll>>>,
    //async ops list
    async_ops: tokio::sync::RwLock<HashMap<String, Arc<AsyncOp>>>,
    //tasks ops
    tasks_ops: tokio::sync::RwLock<HashMap<String, Arc<TaskOp>>>,
}

impl Database {
    /// Is creating a new, fresh database instance withouth
    /// loading from disk
    ///
    /// Is replacing a default implementation, which could not
    /// be added because the Database is used as a dyn Storage
    /// (Self: Sized rule)
    ///
    /// This method should be used if loading failed
    pub(crate) fn new(storage_folder: String) -> Self {
        Self {
            storage_folder,
            applications: RwLock::new(HashMap::new()),
            tasks: RwLock::new(HashMap::new()),
            resources: RwLock::new(HashMap::new()),
            polls: RwLock::new(Vec::new()),
            async_ops: RwLock::new(HashMap::new()),
            tasks_ops: RwLock::new(HashMap::new()),
        }
    }

    /// Is loading the database from the disk
    ///
    /// If file location of any data is not found, a fresh instance will be used.
    ///
    /// # Error
    ///
    /// If failed to load the failed due to unrecoverable errors (eg: failed to serialize)
    /// an error will be returned
    pub(crate) async fn load(storage_folder: String) -> Result<Self, TraceError> {
        // Load all applications
        let applications: HashMap<Uuid, Arc<Application>> =
            match Application::load_all(storage_folder.clone()).await {
                Ok(apps) => apps
                    .into_iter()
                    .map(|(id, app)| (id, Arc::new(app)))
                    .collect(),
                Err(error) => match error {
                    TraceError::PathNotFound(_) => {
                        debug!("Applications file not found, using empty list");
                        HashMap::new()
                    }
                    _ => {
                        error!("Failed to load applications due to {error:?}");
                        return Err(error);
                    }
                },
            };
        debug!(
            "Successfully loaded {} applications from disk.",
            applications.values().len()
        );

        // Load all tasks
        let tasks: HashMap<String, Arc<Task>> = match Task::load_all(storage_folder.clone()).await {
            Ok(tasks) => tasks
                .into_iter()
                .map(|(id, task)| (id, Arc::new(task)))
                .collect(),
            Err(error) => match error {
                TraceError::PathNotFound(_) => {
                    debug!("Tasks file not found, using empty list");
                    HashMap::new()
                }
                _ => {
                    error!("Failed to load tasks due to {error:?}");
                    return Err(error);
                }
            },
        };
        debug!(
            "Successfully loaded {} tasks from disk.",
            tasks.values().len()
        );

        //Load all resources
        let resources: HashMap<String, Arc<Resource>> =
            match Resource::load_all(storage_folder.clone()).await {
                Ok(resources) => resources
                    .into_iter()
                    .map(|(id, resource)| (id, Arc::new(resource)))
                    .collect(),
                Err(error) => match error {
                    TraceError::PathNotFound(_) => {
                        debug!("Tasks file not found, using empty list");
                        HashMap::new()
                    }
                    _ => {
                        error!("Failed to load resources due to {error:?}");
                        return Err(error);
                    }
                },
            };
        debug!(
            "Successfully loaded {} resources from disk.",
            resources.values().len()
        );

        //Load all polls
        let polls: Vec<Arc<Poll>> = match Poll::load_all(storage_folder.clone()).await {
            Ok(polls) => polls.into_iter().map(|poll| Arc::new(poll)).collect(),
            Err(error) => match error {
                TraceError::PathNotFound(_) => {
                    debug!("Polls file not found, using empty list");
                    Vec::new()
                }
                _ => {
                    error!("Failed to load polls due to {error:?}");
                    return Err(error);
                }
            },
        };
        debug!(
            "Successfully loaded {} polls from disk.",
            resources.values().len()
        );

        let async_ops: HashMap<String, Arc<AsyncOp>> =
            match AsyncOp::load_all(storage_folder.clone()).await {
                Ok(async_ops) => async_ops
                    .into_iter()
                    .map(|(id, async_op)| (id, Arc::new(async_op)))
                    .collect(),
                Err(error) => match error {
                    TraceError::PathNotFound(_) => {
                        debug!("Async_op file not found, using empty list");
                        HashMap::new()
                    }
                    _ => {
                        error!("Failed to load polls due to {error:?}");
                        return Err(error);
                    }
                },
            };

        let tasks_ops = match TaskOp::load_all(storage_folder.clone()).await {
            Ok(tasks_ops) => tasks_ops
                .into_iter()
                .map(|(id, task_op)| (id, Arc::new(task_op)))
                .collect(),
            Err(error) => match error {
                TraceError::PathNotFound(_) => {
                    debug!("Tasks_ops file not found, using empty list");
                    HashMap::new()
                }
                _ => {
                    error!("Failed to load polls due to {error:?}");
                    return Err(error);
                }
            },
        };

        Ok(Self {
            storage_folder,
            applications: RwLock::new(applications),
            tasks: RwLock::new(tasks),
            resources: RwLock::new(resources),
            polls: RwLock::new(polls),
            async_ops: RwLock::new(async_ops),
            tasks_ops: RwLock::new(tasks_ops),
        })
    }
}

#[async_trait]
impl Storage for Database {
    async fn applications_read(&self) -> HashMap<Uuid, Arc<Application>> {
        self.applications.read().await.clone()
    }

    async fn applications_write(
        &self,
    ) -> WriteableDataBaseGuard<'_, HashMap<Uuid, Arc<Application>>> {
        let elements = self.applications.write().await;

        WriteableDataBaseGuard {
            folder: &self.storage_folder,
            title: "applications",
            elements,
        }
    }

    async fn tasks_read(&self) -> HashMap<String, Arc<Task>> {
        self.tasks.read().await.clone()
    }

    async fn tasks_write(&self) -> WriteableDataBaseGuard<'_, HashMap<String, Arc<Task>>> {
        let elements = self.tasks.write().await;

        WriteableDataBaseGuard {
            folder: &self.storage_folder,
            title: "tasks",
            elements,
        }
    }

    async fn resources_read(&self) -> HashMap<String, Arc<Resource>> {
        self.resources.read().await.clone()
    }

    async fn resources_write(&self) -> WriteableDataBaseGuard<'_, HashMap<String, Arc<Resource>>> {
        let elements = self.resources.write().await;

        WriteableDataBaseGuard {
            folder: &self.storage_folder,
            title: "resources",
            elements,
        }
    }

    async fn polls_read(&self) -> Vec<Arc<Poll>> {
        self.polls.read().await.clone()
    }

    async fn polls_write(&self) -> WriteableDataBaseGuard<'_, Vec<Arc<Poll>>> {
        let elements = self.polls.write().await;

        WriteableDataBaseGuard {
            folder: &self.storage_folder,
            title: "polls",
            elements,
        }
    }

    async fn async_ops_read(&self) -> HashMap<String, Arc<AsyncOp>> {
        self.async_ops.read().await.clone()
    }

    async fn async_ops_write(&self) -> WriteableDataBaseGuard<'_, HashMap<String, Arc<AsyncOp>>> {
        let elements = self.async_ops.write().await;

        WriteableDataBaseGuard {
            folder: &self.storage_folder,
            title: "async_ops",
            elements,
        }
    }

    async fn tasks_ops_read(&self) -> HashMap<String, Arc<TaskOp>> {
        self.tasks_ops.read().await.clone()
    }

    async fn tasks_ops_write(&self) -> WriteableDataBaseGuard<'_, HashMap<String, Arc<TaskOp>>> {
        let elements = self.tasks_ops.write().await;

        WriteableDataBaseGuard {
            folder: &self.storage_folder,
            title: "tasks_ops",
            elements,
        }
    }
}
