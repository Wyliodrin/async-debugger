//! Persistent, in-memory database backed by disk storage.
//!
//! The `Database` struct holds all domain objects in RAM (wrapped in `Arc<…>`),
//! and syncs them to disk via the `Storage` trait’s read/write guards.
//!
//! # Storage Layout
//!
//! - applications → `<storage_folder>/applications.json`  
//! - tasks        → `<storage_folder>/tasks.json`  
//! - resources    → `<storage_folder>/resources.json`  
//! - polls        → `<storage_folder>/polls.json`  
//! - async_ops    → `<storage_folder>/async_ops.json`  
//! - tasks_ops    → `<storage_folder>/tasks_ops.json`  
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
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use tokio::sync::RwLock;
use uuid::Uuid;

/// In-memory, thread-safe cache of all domain data, persisted on disk.
///
/// Wraps each entity in `Arc<…>` so that clones are cheap, and guards writes
/// behind `tokio::sync::RwLock`. Implements `Storage` to provide atomic read
/// and write handles that flush changes to disk on drop.
#[derive(Default)]
pub(crate) struct Database {
    /// Root folder path for JSON files.
    storage_folder: String,

    /// All persisted applications, keyed by UUID.
    applications: RwLock<HashMap<Uuid, Arc<Application>>>,

    /// All persisted tasks, keyed by their string ID.
    tasks: RwLock<HashMap<String, Arc<Task>>>,

    active_tasks: RwLock<HashSet<String>>,

    /// All persisted resources, keyed by their string ID.
    resources: RwLock<HashMap<String, Arc<Resource>>>,

    /// All persisted polls.
    polls: RwLock<Vec<Arc<Poll>>>,

    /// All persisted asynchronous operations, keyed by string ID.
    async_ops: RwLock<HashMap<String, Arc<AsyncOp>>>,

    /// All persisted task operations, keyed by string ID.
    tasks_ops: RwLock<HashMap<String, Arc<TaskOp>>>,
}

impl Database {
    /// Create a fresh, empty database without attempting to load from disk.
    ///
    /// Use this if you want to start with no pre-existing data.
    ///
    /// # Parameters
    /// - `storage_folder`: path to the folder where JSON files will be read/written.
    pub(crate) fn new(storage_folder: String) -> Self {
        Self {
            storage_folder,
            applications: RwLock::new(HashMap::new()),
            tasks: RwLock::new(HashMap::new()),
            active_tasks: RwLock::new(HashSet::new()),
            resources: RwLock::new(HashMap::new()),
            polls: RwLock::new(Vec::new()),
            async_ops: RwLock::new(HashMap::new()),
            tasks_ops: RwLock::new(HashMap::new()),
        }
    }

    /// Load all persisted data from disk into memory.
    ///
    /// For each entity type, attempts to read `<storage_folder>/<title>.json`.
    /// - If the file is missing, logs a debug‐level message and continues with an empty collection.
    /// - On any other error (I/O, serialization, etc.), returns `Err(TraceError)`.
    ///
    /// # Errors
    /// Returns a `TraceError` if any non-`PathNotFound` error occurs during loading.
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

        let mut active_tasks = HashSet::new();

        for (key, task) in tasks.clone() {
            if !task.is_completed() {
                active_tasks.insert(key);
            }
        }

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

        // Load tasks_ops
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
        debug!("Loaded {} tasks_ops", tasks_ops.len());

        Ok(Self {
            storage_folder,
            applications: RwLock::new(applications),
            tasks: RwLock::new(tasks),
            active_tasks: RwLock::new(active_tasks),
            resources: RwLock::new(resources),
            polls: RwLock::new(polls),
            async_ops: RwLock::new(async_ops),
            tasks_ops: RwLock::new(tasks_ops),
        })
    }
}

#[async_trait]
impl Storage for Database {
    /// Read-only snapshot of all applications.
    async fn applications_read(&self) -> HashMap<Uuid, Arc<Application>> {
        self.applications.read().await.clone()
    }

    /// Obtain a write guard for applications. On drop, writes back to disk.
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

    /// Read-only snapshot of all tasks.
    async fn tasks_read(&self) -> HashMap<String, Arc<Task>> {
        self.tasks.read().await.clone()
    }

    /// Obtain a write guard for tasks. On drop, writes back to disk.
    async fn tasks_write(&self) -> WriteableDataBaseGuard<'_, HashMap<String, Arc<Task>>> {
        let elements = self.tasks.write().await;

        WriteableDataBaseGuard {
            folder: &self.storage_folder,
            title: "tasks",
            elements,
        }
    }

    async fn active_tasks_write(&self) -> tokio::sync::RwLockWriteGuard<'_, HashSet<String>> {
        self.active_tasks.write().await
    }

    async fn active_tasks_read(&self) -> HashSet<String> {
        self.active_tasks.read().await.clone()
    }

    /// Read-only snapshot of all resources.
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

    /// Read-only snapshot of all polls.
    async fn polls_read(&self) -> Vec<Arc<Poll>> {
        self.polls.read().await.clone()
    }

    /// Obtain a write guard for polls. On drop, writes back to disk.
    async fn polls_write(&self) -> WriteableDataBaseGuard<'_, Vec<Arc<Poll>>> {
        let elements = self.polls.write().await;

        WriteableDataBaseGuard {
            folder: &self.storage_folder,
            title: "polls",
            elements,
        }
    }

    /// Read-only snapshot of all asynchronous operations.
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

    /// Read-only snapshot of all task operations.
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
