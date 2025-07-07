use super::connection_manager::{AppUpdate, Connection};
use super::database::Database;
use crate::common::{get_correct_subdivision_sec, get_pid_hosting_at};
use crate::domain::application::{ApplicationState, ConnectionStatus};
use crate::domain::{TaskDuration, TaskState};
use crate::error::Error as TraceError;
use crate::infra::guard::DataBaseWrite;
use crate::infra::storage::Storage;
use crate::{
    domain::{application::Application, Task},
    mappers::tasks::map_to_domain_task,
};
use chrono::{DateTime, Utc};
use console_api::tasks::TaskUpdate;
use log::{debug, error, info, warn};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
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
        //debug for missed task_updates
        if task_update.dropped_events > 0 {
            println!("missed task updates: {:?}", task_update.dropped_events);
        }

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

            // Updating tasks
            for (tid, updated_task) in task_update.stats_update {
                let mut tasks_guard = self.database.tasks_write().await;
                let key = format!("{}.{}", app.title(), tid);
                if let Some(task_arc) = tasks_guard.get_mut(&key) {
                    let task = Arc::make_mut(task_arc);

                    //handle busy time
                    if let Some(poll_stats) = updated_task.poll_stats {
                        if let Some(dur) = poll_stats.busy_time {
                            task.busy = Some(TaskDuration {
                                seconds: dur.seconds,
                                nanos: dur.nanos,
                                formatted: format!(
                                    "{}s {}",
                                    dur.seconds,
                                    get_correct_subdivision_sec(dur.nanos)
                                ),
                            });
                        }
                    }

                    //handle runtime and task status
                    if let Some(dropped_at) = updated_task.dropped_at {
                        // mark it Stopped if it was still Running
                        if matches!(task.state, TaskState::Running) {
                            info!("Marking task {} as Stopped", key);
                            task.state = TaskState::Stopped {
                                at: Utc::now(),
                                reason: None,
                            };
                        }
                        if let Some(created_at) = updated_task.created_at {
                            task.runtime = {
                                let mut seconds = dropped_at.seconds - created_at.seconds;
                                let mut nano = dropped_at.nanos - created_at.nanos;
                                if nano < 0 {
                                    seconds -= 1;
                                    nano = 1_000_000_000 + nano;
                                }
                                Some(TaskDuration {
                                    seconds: seconds,
                                    nanos: nano,
                                    formatted: format!(
                                        "{}s {}",
                                        seconds,
                                        get_correct_subdivision_sec(nano)
                                    ),
                                })
                            };
                        }
                    } else {
                        let now = SystemTime::now();
                        let duration_since_epoch =
                            now.duration_since(UNIX_EPOCH).expect("Time went backwards");

                        if let Some(created_at) = updated_task.created_at {
                            task.runtime = {
                                let mut seconds =
                                    (duration_since_epoch.as_secs() as i64) - created_at.seconds;
                                let mut nano =
                                    (duration_since_epoch.subsec_nanos() as i32) - created_at.nanos;

                                if nano < 0 {
                                    seconds -= 1;
                                    nano = 1000000000 + nano;
                                }

                                Some(TaskDuration {
                                    seconds: seconds,
                                    nanos: nano,
                                    formatted: format!(
                                        "{}s {}",
                                        seconds,
                                        get_correct_subdivision_sec(nano)
                                    ),
                                })
                            };
                        }
                    }

                    //handle schedule time
                    if let Some(scheduled) = updated_task.scheduled_time {
                        task.scheduled = Some(TaskDuration {
                            seconds: scheduled.seconds,
                            nanos: scheduled.nanos,
                            formatted: format!(
                                "{}s {}",
                                scheduled.seconds,
                                get_correct_subdivision_sec(scheduled.nanos)
                            ),
                        });
                    }

                    //handle idle time
                    task.idle = {
                        let idle = match task.runtime.clone() {
                            Some(runtime_duration) => {
                                let mut seconds = runtime_duration.seconds;
                                let mut nanos = runtime_duration.nanos;
                                if let Some(busy_duration) = task.busy.clone() {
                                    if let Some(schedule_duration) = task.scheduled.clone() {
                                        seconds = seconds
                                            - busy_duration.seconds
                                            - schedule_duration.seconds;
                                        nanos =
                                            nanos - busy_duration.nanos - schedule_duration.nanos;
                                    } else {
                                        seconds = seconds - busy_duration.seconds;
                                        nanos = nanos - busy_duration.nanos;
                                    }
                                }
                                while nanos < 0 {
                                    seconds -= 1;
                                    nanos += 1000000000;
                                }
                                Some(TaskDuration {
                                    seconds,
                                    nanos,
                                    formatted: format!(
                                        "{}s {}",
                                        seconds,
                                        get_correct_subdivision_sec(nanos)
                                    ),
                                })
                            }
                            None => None,
                        };
                        idle
                    };

                    //handle created_at
                    if task.created_at.is_none() {
                        task.created_at = DateTime::from_timestamp(
                            updated_task.created_at.unwrap().seconds,
                            updated_task.created_at.unwrap().nanos.try_into().unwrap(),
                        );
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
