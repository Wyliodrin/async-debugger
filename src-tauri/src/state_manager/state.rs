use super::database::Database;
use crate::domain::application::ApplicationState;
use crate::error::Error as TraceError;
use crate::infra::guard::DataBaseWrite;
use crate::infra::storage::Storage;
use crate::{
    domain::{application::Application, Task},
    mappers::tasks::map_to_domain_task,
};
use console_api::tasks::TaskUpdate;
use log::{error, info, warn};
use std::sync::Arc;
use tokio::fs;
use uuid::Uuid;

/// Is managing the access to the database and provides access method
/// tailored for the applications business locic needs
pub struct State {
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

    pub async fn disable_app(&self, uuid: Uuid) -> Result<(), TraceError> {
        let mut guard = self.database.applications_write().await;

        if let Some((_uuid, application)) = guard
            .iter_mut()
            .find(|(app_uuid, _app)| app_uuid.eq(&&uuid))
        {
            let app = application.writeable();
            app.disable().await;
        }

        Ok(())
    }

    pub async fn delete_app(&self, uuid: Uuid) {
        self.database.applications_write().await.remove(&uuid);
    }

    // endregion

    // region TASKS

    pub async fn handle_task_update(&self, app_id: Uuid, task_update: TaskUpdate) {
        if let Some(app) = self.database.applications_read().await.get(&app_id) {
            if app.state() == ApplicationState::Disabled {
                // If app is disabled we dont save anything
                return;
            }

            // Saviing new tasks
            for task in task_update.new_tasks {
                // SAFETY: It should be ok to call unwrap here as any new task will have some stats
                // about when it was spawned
                if let Some(task) = map_to_domain_task(
                    app_id,
                    &task,
                    &task_update.stats_update.get(&task.id.unwrap().id).unwrap(),
                ) {
                    info!("Received a new task for application with id {app_id}");
                    self.database
                        .tasks_write()
                        .await
                        .insert(task.id(), Arc::new(task));
                }
            }

            // Saving dropped tasks
            for (tid, updated_task) in task_update.stats_update {
                if updated_task.dropped_at.is_some() {
                    info!("A task was dropped for application {app_id}");
                    self.database
                        .tasks_write()
                        .await
                        .remove(&format!("{}.{}", app_id, tid));
                }
            }
        } else {
            warn!("Received an update for an app that is not registered");
        }
    }

    pub async fn get_tasks(&self) -> Vec<Arc<Task>> {
        self.database.tasks_read().await.values().cloned().collect()
    }

    // endregion
}
