use super::connection_manager::{AppUpdate, Connection};
use super::database::Database;
use crate::common::get_pid_hosting_at;
use crate::domain::application::{ApplicationState, ConnectionStatus};
use crate::domain::async_op::{CPUOverview, TaskOp, TimeStamp};
use crate::domain::resource::ResourceStatus;
use crate::domain::{duration::Duration, TaskState};
use crate::error::Error as TraceError;
use crate::infra::guard::DataBaseWrite;
use crate::infra::storage::Storage;
use crate::{
    domain::{application::Application, poll::Poll, resource::Resource, Task},
    mappers::{
        async_ops::map_to_domain_async_op, poll::map_to_domain_poll,
        resources::map_to_domain_resource, tasks::map_to_domain_task,
    },
};
use chrono::{DateTime, Local, TimeZone, Utc};
use console_api::async_ops::AsyncOpUpdate;
use console_api::resources::ResourceUpdate;
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

    pub async fn get_application_name_by_id(&self, id: &Uuid) -> Option<String> {
        if let Some(app) = self.database.applications_read().await.get(&id) {
            Some(app.title().to_string())
        } else {
            None
        }
    }

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
                            task.busy = Some(Duration::new(dur.seconds, dur.nanos));
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
                            if let Some(created_at) = updated_task.created_at {
                                task.runtime = {
                                    let mut seconds = dropped_at.seconds - created_at.seconds;
                                    let mut nano = dropped_at.nanos - created_at.nanos;
                                    if nano < 0 {
                                        seconds -= 1;
                                        nano = 1_000_000_000 + nano;
                                    }
                                    Some(Duration::new(seconds, nano))
                                };
                            }
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

                                Some(Duration::new(seconds, nano))
                            };
                        }
                    }

                    //handle schedule time
                    if let Some(scheduled) = updated_task.scheduled_time {
                        task.scheduled = Some(Duration::new(scheduled.seconds, scheduled.nanos))
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
                                Some(Duration::new(seconds, nanos))
                            }
                            None => None,
                        };
                        idle
                    };

                    //handle created_at
                    if task.created_at.is_none() {
                        let created_ts = updated_task
                            .created_at
                            .as_ref()
                            .expect("we just tested is_some()");
                        let dt_local: DateTime<Local> = Local
                            .timestamp_opt(created_ts.seconds, created_ts.nanos as u32)
                            .single()
                            .expect("timestamp invalid");

                        let pretty = format!(
                            r#"<div class="timestamp-chips">
                                <span class="timestamp-chip timestamp-chip--date">{}</span>
                                <span class="timestamp-chip timestamp-chip--time">{}:<span class="timestamp-chip--seconds">{}</span><span class="timestamp-chip--ms">.{}</span></span>
                            </div>"#,
                            dt_local.format("%d/%m/%y"),
                            dt_local.format("%H:%M"),
                            dt_local.format("%S"),
                            dt_local.format("%f")
                        );
                        task.created_at = Some(pretty);
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

    pub async fn rename_task(
        &self,
        task_id: u64,
        task_name: String,
        task_color: String,
        app_name: String,
    ) {
        let key = format!("{}.{}", app_name, task_id);
        let mut tasks = self.database.tasks_write().await;
        if let Some(task_arc) = tasks.get_mut(&key) {
            let task = Arc::make_mut(task_arc);
            task.name = Some(task_name.clone());
            task.color = Some(task_color.clone());
        }

        let mut polls = self.database.polls_write().await;
        polls
            .iter_mut()
            .filter(|poll| poll.task_id == Some(task_id))
            .for_each(|poll_arc| {
                let mut updated_poll = (**poll_arc).clone();
                updated_poll.task_name = Some(task_name.clone());
                updated_poll.task_color = Some(task_color.clone());
                *poll_arc = Arc::new(updated_poll);
            });

        if let Some(task_op_arc) = self.database.tasks_ops_write().await.get_mut(&key) {
            let task_op = Arc::make_mut(task_op_arc);
            task_op.task_name = Some(task_name.clone());
            task_op.task_color = Some(task_color.clone());
        }
    }

    // endregion

    //region RESOURCES + Polls

    pub async fn handle_resource_update(
        &self,
        app_id: Uuid,
        resources_update: ResourceUpdate,
        received_at: Option<String>,
    ) {
        //debug for missed resources_updates
        if resources_update.dropped_events > 0 {
            println!(
                "missed resources updates: {:?}",
                resources_update.dropped_events
            );
        }

        if let Some(app) = self.database.applications_read().await.get(&app_id) {
            if app.state() == ApplicationState::Disabled {
                // If app is disabled we dont save anything
                return;
            }

            // Saving new resources
            for raw in resources_update.new_resources {
                if let Some(mut domain_resource) = map_to_domain_resource(&raw) {
                    domain_resource.app_name = Some(app.title().to_string());
                    info!(
                        "Received a new resource for app '{}' (id {})",
                        app.title(),
                        app_id
                    );
                    self.database
                        .resources_write()
                        .await
                        .insert(domain_resource.id(), Arc::new(domain_resource));
                }
            }

            //Updating Resources
            for (id, updated_resource) in resources_update.stats_update {
                let mut resource_guard = self.database.resources_write().await;
                let key = format!("{}.{}", app.title(), id);
                if let Some(resource_arc) = resource_guard.get_mut(&key) {
                    let resource = Arc::make_mut(resource_arc);

                    //handle duration resource
                    if let Some(dropped_at) = updated_resource.dropped_at {
                        // mark it Stopped if it was still Running
                        if !matches!(resource.status, ResourceStatus::Dropped) {
                            info!("Marking resource {} as Stopped", key);
                            resource.status = ResourceStatus::Dropped;
                            if let Some(created_at) = updated_resource.created_at {
                                resource.duration = {
                                    let mut seconds = dropped_at.seconds - created_at.seconds;
                                    let mut nano = dropped_at.nanos - created_at.nanos;
                                    if nano < 0 {
                                        seconds -= 1;
                                        nano = 1_000_000_000 + nano;
                                    }
                                    Some(Duration::new(seconds, nano))
                                };
                            }
                        }
                    } else {
                        let now = SystemTime::now();
                        let duration_since_epoch =
                            now.duration_since(UNIX_EPOCH).expect("Time went backwards");

                        if let Some(created_at) = updated_resource.created_at {
                            resource.duration = {
                                let mut seconds =
                                    (duration_since_epoch.as_secs() as i64) - created_at.seconds;
                                let mut nano =
                                    (duration_since_epoch.subsec_nanos() as i32) - created_at.nanos;

                                if nano < 0 {
                                    seconds -= 1;
                                    nano = 1000000000 + nano;
                                }

                                Some(Duration::new(seconds, nano))
                            };
                        }
                    }

                    //handle attributes
                    let mut attribute_str = String::from("");
                    for attr in updated_resource.attributes {
                        if let Some(field) = attr.field {
                            if let Some(name) = field.name {
                                match name {
                                    //if the name is a String, we concatenate directly
                                    console_api::field::Name::StrName(s) => {
                                        attribute_str = attribute_str + &s;
                                    }

                                    //if attribute name is an index from metadata.field_names
                                    console_api::field::Name::NameIdx(_) => {
                                        //TODO - a se vedea cum functioneaza NameIdx
                                    }
                                };

                                if let Some(value) = field.value {
                                    attribute_str = attribute_str + ": ";

                                    match value {
                                        console_api::field::Value::DebugVal(val) => {
                                            attribute_str = attribute_str + &val
                                        }
                                        console_api::field::Value::StrVal(val) => {
                                            attribute_str = attribute_str + &val
                                        }
                                        console_api::field::Value::U64Val(val) => {
                                            attribute_str = attribute_str + (&val.to_string())
                                        }
                                        console_api::field::Value::I64Val(val) => {
                                            attribute_str = attribute_str + (&val.to_string())
                                        }
                                        console_api::field::Value::BoolVal(val) => {
                                            attribute_str = attribute_str + (&val.to_string())
                                        }
                                    }
                                }

                                if let Some(s) = attr.unit {
                                    attribute_str = attribute_str + &s;
                                }
                            }
                        }
                        attribute_str = attribute_str + "\n";
                    }

                    if !attribute_str.is_empty() {
                        resource.attributes = Some(attribute_str);
                    }
                }
            }

            //Saving new poll_ops
            for raw in resources_update.new_poll_ops {
                if let Some(mut domain_poll) = map_to_domain_poll(&raw) {
                    domain_poll.app_name = Some(app.title().to_string());
                    domain_poll.received_at = received_at.clone();

                    if let Some(resource_id) = domain_poll.resource_id {
                        let key = format!("{}.{}", app.title(), resource_id);
                        let resources = self.database.resources_read().await;
                        if let Some(resource_arc) = resources.get(&key) {
                            if let Some(loc) = resource_arc.location.clone() {
                                domain_poll.location = Some(loc);
                            }
                            if let Some(name) = resource_arc.target.clone() {
                                domain_poll.resource_name = Some(name);
                            }
                        }

                        info!(
                            "Received a new poll_op for app '{}' (id {})",
                            app.title(),
                            app_id
                        );
                    }

                    if let Some(task_id) = domain_poll.task_id {
                        let key = format!("{}.{}", app.title(), task_id);
                        let tasks = self.database.tasks_read().await;
                        if let Some(task_arc) = tasks.get(&key) {
                            if let Some(task_name) = task_arc.name.clone() {
                                domain_poll.task_name = Some(task_name);
                            }
                        }
                    }

                    self.database
                        .polls_write()
                        .await
                        .push(Arc::new(domain_poll));
                }
            }
        }
    }

    pub async fn get_resources(&self) -> Vec<Arc<Resource>> {
        self.database
            .resources_read()
            .await
            .values()
            .cloned()
            .collect()
    }

    pub async fn get_polls(&self) -> Vec<Arc<Poll>> {
        self.database.polls_read().await
    }
    //endregion

    //region Async op
    pub async fn handle_async_op_update(&self, app_id: Uuid, async_op_update: AsyncOpUpdate) {
        if async_op_update.dropped_events > 0 {
            println!(
                "missed async_op updates: {:?}",
                async_op_update.dropped_events
            );
        }

        if let Some(app) = self.database.applications_read().await.get(&app_id) {
            if app.state() == ApplicationState::Disabled {
                // If app is disabled we dont save anything
                return;
            }

            for raw in async_op_update.new_async_ops {
                if let Some(mut domain_async_op) = map_to_domain_async_op(&raw) {
                    let key = format!("{}.{}", app.title(), domain_async_op.resource_id);
                    if let Some(resource) = self.database.resources_read().await.get(&key) {
                        domain_async_op.resource_target = resource.target.clone();
                        self.database.async_ops_write().await.insert(
                            format!("{}.{}", app.title(), domain_async_op.id),
                            Arc::new(domain_async_op),
                        );
                    }
                }
            }

            for (id, updated_async_op) in async_op_update.stats_update {
                let key = format!("{}.{}", app.title(), id);
                let resource_target;

                if let Some(async_op) = self.database.async_ops_read().await.get(&key) {
                    let resource_id = async_op.resource_id;
                    let key = format!("{}.{}", app.title(), resource_id);

                    if let Some(resource) = self.database.resources_read().await.get(&key) {
                        resource_target = resource.target.clone();
                    } else {
                        resource_target = None;
                    }
                } else {
                    resource_target = None;
                }
                match (updated_async_op.task_id, updated_async_op.poll_stats) {
                    (Some(task_id), Some(poll_stats)) => {
                        if let Some(started_at) = poll_stats.last_poll_started {
                            let key = format!("{}.{}", app.title(), task_id.id);
                            let mut map = self.database.tasks_ops_write().await;
                            if let Some(task_op) = map.get_mut(&key) {
                                let task_op = Arc::make_mut(task_op);
                                if let Some(last_element) = task_op.operations.last_mut() {
                                    let overview_started_at =
                                        last_element.started_at.as_ref().unwrap();

                                    if started_at.nanos == overview_started_at.nanos
                                        && started_at.seconds == overview_started_at.seconds
                                    {
                                        if poll_stats.last_poll_ended.is_some()
                                            && last_element.stopped_at.is_none()
                                        {
                                            last_element.stopped_at = Some(TimeStamp {
                                                seconds: poll_stats
                                                    .last_poll_ended
                                                    .unwrap()
                                                    .seconds,
                                                nanos: poll_stats.last_poll_ended.unwrap().nanos,
                                            });
                                        } else {
                                            continue;
                                        }
                                    } else {
                                        if poll_stats.last_poll_ended.is_some() {
                                            task_op.operations.push(CPUOverview {
                                                started_at: Some(TimeStamp {
                                                    seconds: started_at.seconds,
                                                    nanos: started_at.nanos,
                                                }),
                                                stopped_at: Some(TimeStamp {
                                                    seconds: poll_stats
                                                        .last_poll_ended
                                                        .unwrap()
                                                        .seconds,
                                                    nanos: poll_stats
                                                        .last_poll_ended
                                                        .unwrap()
                                                        .nanos,
                                                }),
                                                resource_target,
                                            });
                                        } else {
                                            task_op.operations.push(CPUOverview {
                                                started_at: Some(TimeStamp {
                                                    seconds: started_at.seconds,
                                                    nanos: started_at.nanos,
                                                }),
                                                stopped_at: None,
                                                resource_target,
                                            });
                                        }
                                    }
                                }
                            } else {
                                let mut operations = Vec::new();
                                if poll_stats.last_poll_ended.is_some() {
                                    operations.push(CPUOverview {
                                        started_at: Some(TimeStamp {
                                            seconds: started_at.seconds,
                                            nanos: started_at.nanos,
                                        }),
                                        stopped_at: Some(TimeStamp {
                                            seconds: poll_stats.last_poll_ended.unwrap().seconds,
                                            nanos: poll_stats.last_poll_ended.unwrap().nanos,
                                        }),
                                        resource_target,
                                    });
                                } else {
                                    operations.push(CPUOverview {
                                        started_at: Some(TimeStamp {
                                            seconds: started_at.seconds,
                                            nanos: started_at.nanos,
                                        }),
                                        stopped_at: None,
                                        resource_target,
                                    });
                                }

                                let task_name;
                                let task_color;
                                let key = format!("{}.{}", app.title(), task_id.id);
                                if let Some(task) = self.database.tasks_read().await.get(&key) {
                                    task_name = task.name.clone();
                                    task_color = task.color.clone();
                                } else {
                                    task_name = None;
                                    task_color = None;
                                }

                                let task_op = TaskOp {
                                    task_id: task_id.id,
                                    task_name,
                                    task_color,
                                    operations: operations.clone(),
                                };

                                map.insert(
                                    format!("{}.{}", app.title(), task_id.id),
                                    Arc::new(task_op),
                                );
                            }
                        }
                    }
                    _ => continue,
                };
            }
        }
    }

    pub async fn get_tasks_ops(&self) -> Vec<Arc<TaskOp>> {
        self.database
            .tasks_ops_read()
            .await
            .values()
            .cloned()
            .collect()
    }
    //endregion
}
