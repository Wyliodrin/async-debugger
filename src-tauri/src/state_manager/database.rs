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
use chrono::Local;
use log::{debug, error};
use std::path::{Path, PathBuf};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use tokio::fs;
use tokio::io::AsyncWriteExt;
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

    /// Export application instance
    /// to /var/lib/async-tracing/exports/<app title>/<timestamp> as separate JSON files.
    /// Writes to a staging folder then renames for atomicity. Deletes exported entries from permanent DB on success.
    async fn export_app_instance(
        &self,
        title: String,
        comment: String,
    ) -> Result<PathBuf, TraceError> {
        let title = title;
        let comment = comment.trim().to_string();
        let apps_map = self.applications_read().await;
        let app_arc = apps_map.values()
            .find(|a| a.title == title)
            .cloned()
            .ok_or_else(|| TraceError::PathNotFound(format!("Application {} not found", title)))?;
        let app_id = app_arc.id();
        let raw_title = app_arc.title().to_string();
        let app_dir_name = {
            if raw_title.is_empty() {
                format!("app-{}", app_id)
            } else {
                raw_title
            }
        };

        let exports_base = Path::new(&self.storage_folder).join("exports");
        let app_folder = exports_base.join(&app_dir_name);

        let ts = Local::now().format("%d.%m.%Y %H:%M:%S").to_string();
        let timestamp_folder = app_folder.join(&ts);

        let staging = app_folder.join(format!("{}.tmp", &ts));
        let final_folder = timestamp_folder.clone();

        fs::create_dir_all(&staging)
            .await
            .map_err(|e| TraceError::CannotCreateStorage {
                error: anyhow::Error::from(e),
                path: staging.to_string_lossy().to_string(),
            })?;

        let safe_comment = if comment.len() > 0 {
            comment.replace('\0', "").replace("\r\n", "\n")
        } else {
            String::new()
        };
        let mut cfile = fs::File::create(staging.join("comment.txt"))
            .await
            .map_err(|e| TraceError::CannotCreateStorage {
                error: anyhow::Error::from(e),
                path: staging.to_string_lossy().to_string(),
            })?;
        cfile
            .write_all(safe_comment.as_bytes())
            .await
            .map_err(|e| TraceError::CannotCreateStorage {
                error: anyhow::Error::from(e),
                path: staging.to_string_lossy().to_string(),
            })?;

        let apps_map = self.applications_read().await;
        let tasks_map = self.tasks_read().await;
        let resources_map = self.resources_read().await;
        let polls_vec = self.polls_read().await;
        let async_ops_map = self.async_ops_read().await;
        let tasks_ops_map = self.tasks_ops_read().await;

        let app_arc = match apps_map.get(&app_id).cloned() {
            Some(a) => a,
            None => {
                let _ = fs::remove_dir_all(&staging).await;
                return Err(TraceError::PathNotFound(format!(
                    "Application {} not found",
                    app_id
                )));
            }
        };

        let app_title = app_arc.title().to_string();
        let mut tasks_out: HashMap<String, Arc<Task>> = HashMap::new();
        for (k, v) in tasks_map.into_iter() {
            if let Some(name) = &v.app_name {
                if name == &app_title {
                    tasks_out.insert(k, v);
                }
            }
        }

        let mut resources_out: HashMap<String, Arc<Resource>> = HashMap::new();
        for (k, v) in resources_map.into_iter() {
            if let Some(name) = &v.app_name {
                if name == &app_title {
                    resources_out.insert(k, v);
                }
            }
        }

        let mut polls_out: Vec<Arc<Poll>> = Vec::new();
        for poll in polls_vec.into_iter() {
            if let Some(name) = &poll.app_name {
                if name == &app_title {
                    polls_out.push(poll);
                }
            }
        }

        let mut async_ops_out: HashMap<String, Arc<AsyncOp>> = HashMap::new();
        for (k, v) in async_ops_map.into_iter() {
            let include = v
                .resource_target
                .as_ref()
                .map(|t| t.contains(&app_title))
                .unwrap_or(false);
            if include {
                async_ops_out.insert(k, v);
            }
        }

        let mut tasks_ops_out: HashMap<String, Arc<TaskOp>> = HashMap::new();
        let exported_task_ids: HashSet<u64> =
            tasks_out.values().filter_map(|t| Some(t.id)).collect();

        for (k, v) in tasks_ops_map.into_iter() {
            if exported_task_ids.contains(&v.task_id) {
                tasks_ops_out.insert(k, v);
            }
        }

        {
            let mut apps_json: HashMap<String, Application> = HashMap::new();
            apps_json.insert(app_arc.id().to_string(), (*Arc::clone(&app_arc)).clone());
            let out = serde_json::to_vec_pretty(&apps_json).map_err(|e| TraceError::Serde(e))?;
            let mut f = fs::File::create(staging.join("applications.json"))
                .await
                .map_err(|e| TraceError::CannotCreateStorage {
                    error: anyhow::Error::from(e),
                    path: staging.to_string_lossy().to_string(),
                })?;
            f.write_all(&out)
                .await
                .map_err(|e| TraceError::CannotCreateStorage {
                    error: anyhow::Error::from(e),
                    path: staging.to_string_lossy().to_string(),
                })?;
        }

        {
            let mut tasks_json: HashMap<String, Task> = HashMap::new();
            for (k, v) in tasks_out.into_iter() {
                tasks_json.insert(k, (*Arc::clone(&v)).clone());
            }
            let out = serde_json::to_vec_pretty(&tasks_json).map_err(|e| TraceError::Serde(e))?;
            let mut f = fs::File::create(staging.join("tasks.json"))
                .await
                .map_err(|e| TraceError::CannotCreateStorage {
                    error: anyhow::Error::from(e),
                    path: staging.to_string_lossy().to_string(),
                })?;
            f.write_all(&out)
                .await
                .map_err(|e| TraceError::CannotCreateStorage {
                    error: anyhow::Error::from(e),
                    path: staging.to_string_lossy().to_string(),
                })?;
        }

        {
            let mut resources_json: HashMap<String, Resource> = HashMap::new();
            for (k, v) in resources_out.into_iter() {
                resources_json.insert(k, (*Arc::clone(&v)).clone());
            }
            let out =
                serde_json::to_vec_pretty(&resources_json).map_err(|e| TraceError::Serde(e))?;
            let mut f = fs::File::create(staging.join("resources.json"))
                .await
                .map_err(|e| TraceError::CannotCreateStorage {
                    error: anyhow::Error::from(e),
                    path: staging.to_string_lossy().to_string(),
                })?;
            f.write_all(&out)
                .await
                .map_err(|e| TraceError::CannotCreateStorage {
                    error: anyhow::Error::from(e),
                    path: staging.to_string_lossy().to_string(),
                })?;
        }

        {
            let polls_vec_owned: Vec<Poll> = polls_out
                .into_iter()
                .map(|p| (*Arc::clone(&p)).clone())
                .collect();
            let out =
                serde_json::to_vec_pretty(&polls_vec_owned).map_err(|e| TraceError::Serde(e))?;
            let mut f = fs::File::create(staging.join("polls.json"))
                .await
                .map_err(|e| TraceError::CannotCreateStorage {
                    error: anyhow::Error::from(e),
                    path: staging.to_string_lossy().to_string(),
                })?;
            f.write_all(&out)
                .await
                .map_err(|e| TraceError::CannotCreateStorage {
                    error: anyhow::Error::from(e),
                    path: staging.to_string_lossy().to_string(),
                })?;
        }

        {
            let mut async_ops_json: HashMap<String, AsyncOp> = HashMap::new();
            for (k, v) in async_ops_out.into_iter() {
                async_ops_json.insert(k, (*Arc::clone(&v)).clone());
            }
            let out =
                serde_json::to_vec_pretty(&async_ops_json).map_err(|e| TraceError::Serde(e))?;
            let mut f = fs::File::create(staging.join("async_ops.json"))
                .await
                .map_err(|e| TraceError::CannotCreateStorage {
                    error: anyhow::Error::from(e),
                    path: staging.to_string_lossy().to_string(),
                })?;
            f.write_all(&out)
                .await
                .map_err(|e| TraceError::CannotCreateStorage {
                    error: anyhow::Error::from(e),
                    path: staging.to_string_lossy().to_string(),
                })?;
        }

        {
            let mut tasks_ops_json: HashMap<String, TaskOp> = HashMap::new();
            for (k, v) in tasks_ops_out.into_iter() {
                tasks_ops_json.insert(k, (*Arc::clone(&v)).clone());
            }
            let out =
                serde_json::to_vec_pretty(&tasks_ops_json).map_err(|e| TraceError::Serde(e))?;
            let mut f = fs::File::create(staging.join("tasks_ops.json"))
                .await
                .map_err(|e| TraceError::CannotCreateStorage {
                    error: anyhow::Error::from(e),
                    path: staging.to_string_lossy().to_string(),
                })?;
            f.write_all(&out)
                .await
                .map_err(|e| TraceError::CannotCreateStorage {
                    error: anyhow::Error::from(e),
                    path: staging.to_string_lossy().to_string(),
                })?;
        }

        fs::rename(&staging, &final_folder).await.map_err(|e| {
            let _ = futures::executor::block_on(fs::remove_dir_all(&staging));
            TraceError::CannotCreateStorage {
                error: anyhow::Error::from(e),
                path: final_folder.to_string_lossy().to_string(),
            }
        })?;

        {
            let mut apps_guard = self.applications.write().await;
            apps_guard.remove(&app_id);
        }

        {
            let mut tasks_guard = self.tasks.write().await;
            tasks_guard
                .retain(|_k, v| v.app_name.as_ref().map(|n| n != &app_title).unwrap_or(true));
        }

        {
            let mut resources_guard = self.resources.write().await;
            resources_guard
                .retain(|_k, v| v.app_name.as_ref().map(|n| n != &app_title).unwrap_or(true));
        }

        {
            let mut polls_guard = self.polls.write().await;
            polls_guard.retain(|p| p.app_name.as_ref().map(|n| n != &app_title).unwrap_or(true));
        }

        {
            let mut async_guard = self.async_ops.write().await;
            async_guard.retain(|_k, v| {
                v.resource_target
                    .as_ref()
                    .map(|t| !t.contains(&app_title))
                    .unwrap_or(true)
            });
        }

        {
            let mut tasks_ops_guard = self.tasks_ops.write().await;
            tasks_ops_guard.retain(|_k, v| !exported_task_ids.contains(&v.task_id));
        }

        Ok(final_folder)
    }

    async fn import_from_export_folder(&self, folder: PathBuf) -> Result<(), String> {
        if !folder.exists() {
            return Err(format!(
                "Export folder not found: {}",
                folder.to_string_lossy()
            ));
        }

        async fn read_if_exists(p: PathBuf) -> Result<Option<Vec<u8>>, String> {
            if p.exists() {
                fs::read(&p)
                    .await
                    .map(Some)
                    .map_err(|e| format!("Failed to read {}: {}", p.to_string_lossy(), e))
            } else {
                Ok(None)
            }
        }

        if let Some(bytes) = read_if_exists(folder.join("applications.json")).await? {
            let apps_map: HashMap<String, Application> = serde_json::from_slice(&bytes)
                .map_err(|e| format!("applications.json parse error: {}", e))?;
            let mut guard = self.applications_write().await;
            for (k, v) in apps_map.into_iter() {
               if let Ok(id) = uuid::Uuid::parse_str(&k) {
                    guard.insert(id, std::sync::Arc::new(v));
                } else {
                   }
            }
            drop(guard);
        }

        if let Some(bytes) = read_if_exists(folder.join("tasks.json")).await? {
            let tasks_map: HashMap<String, Task> = serde_json::from_slice(&bytes)
                .map_err(|e| format!("tasks.json parse error: {}", e))?;
            let mut guard = self.tasks_write().await;
            for (k, v) in tasks_map.into_iter() {
                guard.insert(k, std::sync::Arc::new(v));
            }
            drop(guard);
        }

        if let Some(bytes) = read_if_exists(folder.join("resources.json")).await? {
            let resources_map: HashMap<String, Resource> = serde_json::from_slice(&bytes)
                .map_err(|e| format!("resources.json parse error: {}", e))?;
            let mut guard = self.resources_write().await;
            for (k, v) in resources_map.into_iter() {
                guard.insert(k, std::sync::Arc::new(v));
            }
            drop(guard);
        }

        if let Some(bytes) = read_if_exists(folder.join("polls.json")).await? {
            let polls_vec: Vec<Poll> = serde_json::from_slice(&bytes)
                .map_err(|e| format!("polls.json parse error: {}", e))?;
            let mut guard = self.polls_write().await;
            for p in polls_vec.into_iter() {
                guard.push(std::sync::Arc::new(p));
            }
            drop(guard);
        }

        if let Some(bytes) = read_if_exists(folder.join("async_ops.json")).await? {
            let async_ops_map: HashMap<String, AsyncOp> = serde_json::from_slice(&bytes)
                .map_err(|e| format!("async_ops.json parse error: {}", e))?;
            let mut guard = self.async_ops_write().await;
            for (k, v) in async_ops_map.into_iter() {
                guard.insert(k, std::sync::Arc::new(v));
            }
            drop(guard);
        }

        if let Some(bytes) = read_if_exists(folder.join("tasks_ops.json")).await? {
            let tasks_ops_map: HashMap<String, TaskOp> = serde_json::from_slice(&bytes)
                .map_err(|e| format!("tasks_ops.json parse error: {}", e))?;
            let mut guard = self.tasks_ops_write().await;
            for (k, v) in tasks_ops_map.into_iter() {
                guard.insert(k, std::sync::Arc::new(v));
            }
            drop(guard);
        }

        Ok(())
    }
}
