use super::connection_manager::{AppUpdate, Connection};
use super::database::Database;
use crate::common::get_pid_hosting_at;
use crate::domain::application::{ApplicationState, ConnectionStatus};
use crate::domain::TaskState;
use crate::error::Error as TraceError;
use crate::infra::guard::DataBaseWrite;
use crate::infra::storage::Storage;
use crate::{
    domain::{application::Application, Task},
    mappers::tasks::map_to_domain_task,
};
use chrono::{DateTime, NaiveDateTime, Utc};
use console_api::tasks::TaskUpdate;
use log::{debug, error, info, warn};
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

        // Check if PIDs have changed, if so update them
        let mut guard = database.applications_write().await;
        for (_uuid, app) in guard.iter_mut() {
            if let Some(pid) = get_pid_hosting_at(app.url().clone()) {
                if app.pid() != pid {
                    debug!("Updating the PID for app {} to {}", app.title(), pid);
                    app.writeable().set_pid(pid);

                    debug!("Checking pid {}", app.pid());
                }
            }
        }
        drop(guard);

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

    pub async fn disable_app(&self, app_id: Uuid) -> Result<(), TraceError> {
        let mut guard = self.database.applications_write().await;

        if let Some((_uuid, application)) =
            guard.iter_mut().find(|(_uuid, app)| app.id().eq(&app_id))
        {
            let app = application.writeable();
            app.disable().await;
        }

        Ok(())
    }

    pub async fn enable_app(&self, app_id: Uuid, connection: Connection) {
        let mut guard = self.database.applications_write().await;

        if let Some((_uuid, application)) =
            guard.iter_mut().find(|(_uuid, app)| app.id().eq(&app_id))
        {
            let app = application.writeable();
            app.enable(connection);
        }
    }

    pub async fn delete_application(&self, app_id: Uuid) {
        self.database.applications_write().await.remove(&app_id);
    }

    // pub async fn edit_application(&self, app_id: Uuid) {

    // }

    // endregion

    // region TASKS

    /// Receives a [`TaskUpdate`] object and applies the updates received
    /// on the current list of tasks
    pub async fn handle_task_update(&self, app_id: Uuid, task_update: TaskUpdate) {
        if let Some(app) = self.database.applications_read().await.get(&app_id) {
            if app.state() == ApplicationState::Disabled {
                // If app is disabled we dont save anything
                return;
            }

            // Saving new tasks
            for raw in task_update.new_tasks {
                if let Some(mut domain_task) = map_to_domain_task(app_id, &raw) {
                    domain_task.app_name = Some(app.title().to_string());

                    info!(
                        "Received a new task for app '{}' (id {})",
                        app.title(),
                        app_id
                    );
                    self.database
                        .tasks_write()
                        .await
                        .insert(domain_task.id(), Arc::new(domain_task));
                }
            }

            // Saving dropped tasks
            let mut tasks_guard = self.database.tasks_write().await;
            for (tid, updated_task) in task_update.stats_update {
                if updated_task.dropped_at.is_some() {
                    let key = format!("{}", tid);
                    println!("{}", key);
                    if let Some(task_arc) = tasks_guard.get_mut(&key) {
                        let task = Arc::make_mut(task_arc);
                        if matches!(task.state, TaskState::Running)
                            && (updated_task.dropped_at.is_some()
                                || updated_task.dropped_at.is_some())
                        {
                            task.state = TaskState::Stopped {
                                at: updated_task
                                    // turn the Option<prost_types::Timestamp> into Option<DateTime<Utc>>
                                    .dropped_at
                                    .or(updated_task.dropped_at)
                                    .map(|ts| {
                                        let naive = NaiveDateTime::from_timestamp(
                                            ts.seconds,
                                            ts.nanos as u32,
                                        );
                                        DateTime::<Utc>::from_utc(naive, Utc)
                                    })
                                    .unwrap_or_else(|| Utc::now()),
                                reason: None,
                            };
                        }
                    }
                }
            }
        }
    }

    /// Receives an update regarding an Application with the given [`app_id`]
    /// The update consists in the new Application object that needs to replace
    /// the old one
    pub async fn handle_app_update(&self, app_id: Uuid, update: AppUpdate) {
        let mut guard = self.database.applications_write().await;
        if let Some((_uuid, app)) = guard.iter_mut().find(|(_uuid, app)| app.id().eq(&app_id)) {
            if app.state() == ApplicationState::Disabled {
                // If app is disabled we dont save anything
                return;
            } else {
                let writeable_app = app.writeable();
                if let Some(cpu_usage) = update.cpu_usage {
                    writeable_app.set_cpu_usage(cpu_usage);
                }
                writeable_app.set_memory_usage(update.memory_usage);
            }
        } else {
            warn!("Received an application update for an app that is not registered");
            return;
        }
    }

    pub async fn handle_app_conn_update(&self, app_id: Uuid, conn_status: ConnectionStatus) {
        if matches!(conn_status, ConnectionStatus::Disconnected)
            || matches!(conn_status, ConnectionStatus::Error(_))
        {
            let mut tasks_guard = self.database.tasks_write().await;
            let prefix = format!("{}.", app_id);
            for (key, task_arc) in tasks_guard.iter_mut() {
                if key.starts_with(&prefix) {
                    let t = Arc::make_mut(task_arc);
                    if matches!(t.state, TaskState::Running) {
                        t.state = TaskState::Stopped {
                            at: Utc::now(),
                            reason: None,
                        };
                    }
                }
            }
        }
        let mut guard = self.database.applications_write().await;
        if let Some((_uuid, app)) = guard.iter_mut().find(|(_uuid, app)| app.id().eq(&app_id)) {
            if app.state() == ApplicationState::Disabled {
                // If app is disabled we dont save anything
                return;
            } else {
                let writeable_app = app.writeable();
                writeable_app.set_connection_status(conn_status);
            }
        } else {
            warn!("Received an application update for an app that is not registered");
            return;
        }
    }

    pub async fn get_tasks(&self) -> Vec<Arc<Task>> {
        self.database.tasks_read().await.values().cloned().collect()
    }

    pub async fn stop_task(&self, task_id: &str) {
        let mut tasks = self.database.tasks_write().await;
        if let Some(task_arc) = tasks.get_mut(task_id) {
            let task = Arc::make_mut(task_arc);
            if matches!(task.state, TaskState::Running) {
                task.state = TaskState::Stopped {
                    at: Utc::now(),
                    reason: None,
                };
            }
        }
    }

    // endregion
}
