use super::database::Database;
use crate::error::Error as TraceError;
use crate::infra::storage::Storage;
use crate::{
    domain::{application::Application, Task},
    mappers::tasks::map_to_domain_task,
};
use console_api::tasks::TaskUpdate;
use log::{error, info};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use uuid::Uuid;

/// Is managing the access to the database and provides access method
/// tailored for the applications business locic needs
pub struct State {
    storage_folder: PathBuf,
    database: Arc<dyn Storage>,
}

impl State {
    const STORAGE_FOLDER: &str = ".async-tracing";

    /// Creates a new, fresh instance
    /// Will not load the database anymore, but use empty lists for every
    /// element
    ///
    /// Should be used in case of failure when loading
    pub fn new() -> Self {
        let path = dirs::home_dir().unwrap().join(Self::STORAGE_FOLDER);
        info!("Storage location is: {path:?}");

        Self {
            storage_folder: path.clone(),
            // TODO: remove unwrap here
            database: Arc::new(Database::new(path.as_path().to_string_lossy().to_string())),
        }
    }

    /// Is loading state from previus application instance
    ///
    /// # Error
    ///
    /// If failed to load data from disk, will return an error
    pub async fn load() -> Result<State, TraceError> {
        let database_path = dirs::home_dir().unwrap().join(Self::STORAGE_FOLDER);
        info!("Storage location is: {database_path:?}");

        // Checking if storage folder exists
        if !database_path.is_dir() {
            // Create the storage folder
            if let Err(error) = fs::create_dir(&database_path).await {
                error!("Could not create the storage folder at path {database_path:?} due to {error:?}");
                return Err(TraceError::CannotCreateStorage {
                    error: error.into(),
                    path: database_path.to_string_lossy().to_string(),
                });
            }
        }

        let database =
            Database::load(database_path.as_path().to_string_lossy().to_string()).await?;

        Ok(State {
            storage_folder: database_path,
            database: Arc::new(database),
        })
    }

    // region APPLICATIONS

    pub async fn get_current_applications_list(&self) -> Vec<Arc<Application>> {
        self.database
            .applications_read()
            .await
            .values()
            .cloned()
            .collect()
    }

    pub async fn store_app(&self, application: Application) {
        self.database
            .applications_write()
            .await
            .insert(application.id().clone(), Arc::new(application));
    }

    pub async fn delete_app(&self, uuid: Uuid) {
        self.database.applications_write().await.remove(&uuid);
    }

    // endregion

    // region TASKS

    pub async fn handle_task_update(&self, app_id: Uuid, task_update: TaskUpdate) {
        // Handling new tasks
        for task in task_update.new_tasks {
            if let Some(task) = map_to_domain_task(app_id, &task) {
                info!("Received a new task for application with id {app_id}");
                self.database
                    .tasks_write()
                    .await
                    .insert(task.id(), Arc::new(task));
            }
        }

        // Handling dropped tasks
        for (tid, updated_task) in task_update.stats_update {
            if updated_task.dropped_at.is_some() {
                info!("A task was dropped for application {app_id}");
                self.database
                    .tasks_write()
                    .await
                    .remove(&format!("{}.{}", app_id, tid));
            }
        }
    }

    pub async fn get_tasks(&self) -> Vec<Arc<Task>> {
        self.database.tasks_read().await.values().cloned().collect()
    }

    // endregion
}
