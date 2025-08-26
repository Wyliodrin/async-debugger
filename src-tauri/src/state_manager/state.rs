use super::connection_manager::{AppUpdate, Connection};
use super::database::Database;
use crate::common::{get_pid_hosting_at, rename_database_keys, rename_database_keys_and_app_name};
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

/// Manages access to persistent storage and provides high-level methods
/// for reading and writing application, task and resource state.
pub struct State {
    database: Arc<dyn Storage>,
}

impl State {
    const STORAGE_FOLDER: &str = ".async-tracing";

    /// Constructs a new `State` backed by an empty in‐memory database.
    ///
    /// This will not load any existing data from disk. It is intended for
    /// use when a previous load operation failed and a fresh start is required.
    pub fn new() -> Self {
        let path = dirs::home_dir().unwrap().join(Self::STORAGE_FOLDER);
        info!("Storage location is: {path:?}");

        Self {
            database: Arc::new(Database::new(path.as_path().to_string_lossy().to_string())),
        }
    }

    /// Loads state from the file system into memory.
    ///
    /// This will create the storage directory if it does not exist,
    /// then load all persisted applications and tasks. Upon success,
    /// it also refreshes process IDs of running applications.
    ///
    /// # Errors
    ///
    /// Returns [`TraceError::CannotCreateStorage`] if the storage directory
    /// cannot be created, or any other error from loading the underlying database.
    pub async fn load() -> Result<State, TraceError> {
        let database_path = dirs::home_dir().unwrap().join(Self::STORAGE_FOLDER);
        info!("Storage location is: {database_path:?}");

        // Checking if storage folder exists
        if !database_path.is_dir() {
            // Create the storage folder
            if let Err(error) = fs::create_dir(&database_path).await {
                error!(
                    "Could not create the storage folder at path {database_path:?} due to {error:?}"
                );
                return Err(TraceError::CannotCreateStorage {
                    error: error.into(),
                    path: database_path.to_string_lossy().to_string(),
                });
            }
        }

        let database =
            Database::load(database_path.as_path().to_string_lossy().to_string()).await?;

        // Refresh PIDs for applications whose host process may have changed
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

    /// Retrieves the title of the application identified by `id`.
    ///
    /// # Returns
    ///
    /// - `Some(String)` containing the application title if found.
    /// - `None` if no application with that ID exists.
    pub async fn get_application_name_by_id(&self, id: &Uuid) -> Option<String> {
        if let Some(app) = self.database.applications_read().await.get(&id) {
            Some(app.title().to_string())
        } else {
            None
        }
    }

    /// Returns a list of all currently stored applications.
    ///
    /// The returned vector contains an `Arc<Application>` for each
    /// application in the database.
    pub(crate) async fn get_current_applications_list(&self) -> Vec<Arc<Application>> {
        self.database
            .applications_read()
            .await
            .values()
            .cloned()
            .collect()
    }

    /// Persists a new application in the database.
    ///
    /// If an application with the same ID already exists, it will be
    /// overwritten.
    pub(crate) async fn store_app(&self, application: Application) {
        self.database
            .applications_write()
            .await
            .insert(application.id().clone(), Arc::new(application));
    }

    /// Disables the application with the given `app_id`.
    ///
    /// This sets the internal state of the application to `Disabled`,
    /// preventing further updates from being recorded.
    ///
    /// # Errors
    ///
    /// Always returns `Ok(())` but may fail silently if the write
    /// lock cannot be acquired.
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

    /// Renames the application title across multiple internal database collections.
    ///
    /// This asynchronous function updates references of an application's title
    /// from `old_title` to `new_title` in various database sections:
    /// - Async operations (`async_ops`)
    /// - Resources (`resources`)
    /// - Tasks (`tasks`)
    /// - Task operations (`tasks_ops`)
    /// - Polls (`polls`)
    ///
    /// For collections that store values implementing the `HasAppName` trait,
    /// the function also updates the `app_name` field accordingly.
    ///
    /// # Parameters
    ///
    /// * `new_title` - The new application title to replace the old one.
    /// * `old_title` - The old application title that needs to be replaced.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if all renaming operations succeed,
    /// or a `TraceError` if any step encounters an error.
    pub async fn edit_app(&self, new_title: String, old_title: String) -> Result<(), TraceError> {
        {
            let mut async_guard = self.database.async_ops_write().await;
            rename_database_keys(&mut async_guard, new_title.clone(), old_title.clone());
        }

        {
            let mut resource_guard = self.database.resources_write().await;
            rename_database_keys_and_app_name(
                &mut resource_guard,
                new_title.clone(),
                old_title.clone(),
            );
        }

        {
            let mut tasks_guard = self.database.tasks_write().await;
            rename_database_keys_and_app_name(
                &mut tasks_guard,
                new_title.clone(),
                old_title.clone(),
            );
        }

        {
            let mut tasks_ops_guard = self.database.tasks_ops_write().await;
            rename_database_keys(&mut tasks_ops_guard, new_title.clone(), old_title.clone());
        }

        {
            let mut poll_guard = self.database.polls_write().await;
            for poll_arc in poll_guard.iter_mut() {
                let poll_mut = Arc::make_mut(poll_arc);
                if poll_mut.app_name == Some(old_title.clone()) {
                    poll_mut.app_name = Some(new_title.clone());
                }
            }
        }
        Ok(())
    }

    /// Enables the application with the given `app_id` and sets its
    /// connection information.
    ///
    /// After enabling, updates for this application will once again
    /// be recorded.
    pub async fn enable_app(&self, app_id: Uuid, connection: Connection) {
        let mut guard = self.database.applications_write().await;

        if let Some((_uuid, application)) =
            guard.iter_mut().find(|(_uuid, app)| app.id().eq(&app_id))
        {
            let app = application.writeable();
            app.enable(connection);
        }
    }

    /// Deletes the application identified by `app_id` from storage.
    ///
    /// All associated tasks remain in the database unless explicitly
    /// removed by other operations.
    pub async fn delete_application(&self, app_id: Uuid) {
        self.database.applications_write().await.remove(&app_id);
    }

    // endregion

    // region TASKS

    /// Applies a batch of task updates for the application `app_id`.
    ///
    /// This method will:
    /// - Insert any new tasks reported in `task_update.new_tasks`.
    /// - Update runtime, busy time, idle time, schedule time, and
    ///   created timestamp for existing tasks.
    /// - Mark tasks as stopped if they have been dropped.
    ///
    /// If the application is currently disabled, updates are ignored.
    pub async fn handle_task_update(&self, app_id: Uuid, task_update: TaskUpdate) -> Vec<String> {
        let mut warnings: Vec<String> = Vec::new();

        // debug for missed task_updates
        if task_update.dropped_events > 0 {
            println!("missed task updates: {:?}", task_update.dropped_events);
        }

        if let Some(app) = self.database.applications_read().await.get(&app_id) {
            if app.state() == ApplicationState::Disabled {
                // If app is disabled we dont save anything
                return Vec::new();
            }

            // Insert new tasks
            for raw in task_update.new_tasks {
                if let Some(mut domain_task) = map_to_domain_task(app_id, &raw) {
                    domain_task.app_name = Some(app.title().to_string());
                    info!(
                        "Received a new task for app '{}' (id {})",
                        app.title(),
                        app_id
                    );

                    {
                        let mut guard = self.database.active_tasks_write().await;
                        guard.insert(domain_task.id().clone());
                    }

                    self.database
                        .tasks_write()
                        .await
                        .insert(domain_task.id(), Arc::new(domain_task));
                }
            }

            let mut active_tasks_clone = self.database.active_tasks_read().await;

            // Update existing tasks
            for (tid, updated_task) in task_update.stats_update {
                let mut tasks_guard = self.database.tasks_write().await;
                let key = format!("{}.{}", app.title(), tid);
                if let Some(task_arc) = tasks_guard.get_mut(&key) {
                    let task = Arc::make_mut(task_arc);

                    // Handle busy time & poll stats
                    if let Some(poll_stats) = updated_task.poll_stats {
                        if let Some(dur) = poll_stats.busy_time {
                            task.busy = Some(Duration::new(dur.seconds, dur.nanos));
                        }

                        task.last_poll_started =
                            poll_stats.last_poll_started.map(|v| v.try_into().unwrap());
                        task.last_poll_ended =
                            poll_stats.last_poll_ended.map(|v| v.try_into().unwrap());
                        task.polls = poll_stats.polls;
                    }

                    // Handle runtime & stop detection
                    if let Some(dropped_at) = updated_task.dropped_at {
                        if matches!(task.state, TaskState::Running) {
                            info!("Marking task {} as Stopped", key);
                            task.state = TaskState::Stopped {
                                at: Utc::now(),
                                reason: None,
                            };
                            {
                                let mut active_tasks_guard =
                                    self.database.active_tasks_write().await;
                                active_tasks_guard.remove(&task.id());
                                active_tasks_clone.remove(&task.id());
                            }
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
                        // Update runtime based on current time
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
                                    nano = 1_000_000_000 + nano;
                                }

                                Some(Duration::new(seconds, nano))
                            };
                        }
                    }

                    // Handle scheduled time
                    if let Some(scheduled) = updated_task.scheduled_time {
                        task.scheduled = Some(Duration::new(scheduled.seconds, scheduled.nanos))
                    }

                    // Compute idle time
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

                    // Format created_at timestamp if not yet set
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

                    task.wakes = updated_task.wakes;
                    task.self_wakes = updated_task.self_wakes;
                    task.waker_clones = updated_task.waker_clones;
                    task.waker_drops = updated_task.waker_drops;

                    //check for warnings
                    warnings.extend(task.check_warnings());
                    active_tasks_clone.remove(&task.id());
                }
            }
            let tasks = self.database.tasks_read().await;
            for id in active_tasks_clone {
                if let Some(task) = tasks.get(&id) {
                    warnings.extend(task.check_warnings());
                }
            }
            warnings
        } else {
            return Vec::new();
        }
    }

    /// Updates CPU and memory usage for the application `app_id`.
    ///
    /// If the application is disabled, this update will be ignored.
    /// If `app_id` is not found, a warning is logged.
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

    /// Updates connection status for the application `app_id`.
    ///
    /// If the status is `Disconnected` or `Error`, all running tasks for
    /// that app are marked as stopped. If the application is disabled,
    /// the update is ignored. If `app_id` is not found, a warning is logged.
    pub(crate) async fn handle_app_conn_update(&self, app_id: Uuid, conn_status: ConnectionStatus) {
        let mut guard = self.database.applications_write().await;
        if let Some((_uuid, app)) = guard.iter_mut().find(|(_uuid, app)| app.id().eq(&app_id)) {
            // For disconnects or errors, stop all running tasks of the app
            if matches!(conn_status, ConnectionStatus::Disconnected)
                || matches!(conn_status, ConnectionStatus::Error(_))
            {
                {
                    let mut tasks_guard = self.database.tasks_write().await;
                    let prefix = format!("{}.", app.title());
                    for (key, task_arc) in tasks_guard.iter_mut() {
                        if key.starts_with(&prefix) {
                            let t = Arc::make_mut(task_arc);
                            if matches!(t.state, TaskState::Running) {
                                t.state = TaskState::Stopped {
                                    at: Utc::now(),
                                    reason: None,
                                };

                                // maybe update the runtime, busy, schedule as well or just runtime
                            }
                        }
                    }
                }

                {
                    let mut resources_guard = self.database.resources_write().await;
                    let prefix = format!("{}.", app.title());
                    for (key, resource_arc) in resources_guard.iter_mut() {
                        if key.starts_with(&prefix) {
                            let r = Arc::make_mut(resource_arc);
                            if matches!(r.status, ResourceStatus::Ready) {
                                r.status = ResourceStatus::Dropped;
                            }
                        }
                    }
                }
            }
            // Update app connection status
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

    /// Update the PID for an application in memory and persist it.
    pub async fn handle_pid_changed(&self, app_id: Uuid, new_pid: u32) {
        let mut apps = self.database.applications_write().await;
        if let Some(app) = apps.get_mut(&app_id) {
            app.writeable().set_pid(new_pid);
        }
        // drop guard
    }

    /// Lookup the current PID for an application.
    pub async fn get_pid_for(&self, app_id: Uuid) -> Option<u32> {
        let apps = self.database.applications_read().await;
        apps.get(&app_id).map(|app| app.pid())
    }

    /// Returns all tasks currently stored in memory.
    pub async fn get_tasks(&self) -> Vec<Arc<Task>> {
        self.database.tasks_read().await.values().cloned().collect()
    }

    /// Stops the task identified by `task_id` if it is currently running.
    ///
    /// The task state is set to `Stopped` with the current UTC timestamp.
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

    /// Renames a task, updates its display color, and propagates the changes
    /// to any related poll records and task operations.
    ///
    /// - `task_id`: The numeric identifier of the task within its application.
    /// - `task_name`: The new human-readable name to assign to the task.
    /// - `task_color`: The new color code (e.g. hex string) to use when rendering the task.
    /// - `app_name`: The title of the application to which the task belongs.
    ///
    /// This method will:
    /// 1. Look up the task in the in-memory task store and update
    ///    its `name` and `color` fields.
    /// 2. Iterate over all persisted polls, matching on `task_id`,
    ///    and update each poll’s `task_name` and `task_color`.
    /// 3. If there is an entry in the task‐operations store matching
    ///    the same key, update its `task_name` and `task_color` as well.
    pub async fn rename_task(
        &self,
        task_id: u64,
        task_name: String,
        task_color: String,
        app_name: String,
    ) {
        let key = format!("{}.{}", app_name, task_id);

        // Update the task record itself
        let mut tasks = self.database.tasks_write().await;
        if let Some(task_arc) = tasks.get_mut(&key) {
            let task = Arc::make_mut(task_arc);
            task.name = Some(task_name.clone());
            task.color = Some(task_color.clone());
        }

        // Update all polls associated with this task
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

        // Update any task-operation entries
        if let Some(task_op_arc) = self.database.tasks_ops_write().await.get_mut(&key) {
            let task_op = Arc::make_mut(task_op_arc);
            task_op.task_name = Some(task_name.clone());
            task_op.task_color = Some(task_color.clone());
        }
    }

    // endregion

    // region RESOURCES + Polls

    /// Processes a batch of resource updates and new poll operations for
    /// a given application.
    ///
    /// - `app_id`: The UUID of the application reporting the update.
    /// - `resources_update`: The incoming `ResourceUpdate` event payload.
    /// - `received_at`: An optional timestamp string indicating when the
    ///    update was received (used for labeling new polls).
    ///
    /// This method will:
    /// 1. Insert any new resources into the in-memory resource store,
    ///    tagging them with the application name.
    /// 2. Update existing resources’ status, duration, and attribute
    ///    fields based on the `stats_update` section.
    /// 3. Convert any new poll operations into domain `Poll` objects,
    ///    enriching them with known resource location, name, and task
    ///    metadata, then append them to the poll log.
    ///
    /// If the application is currently disabled, all updates are ignored.
    pub async fn handle_resource_update(
        &self,
        app_id: Uuid,
        resources_update: ResourceUpdate,
        received_at: Option<String>,
    ) {
        // debug for missed resources_updates
        if resources_update.dropped_events > 0 {
            println!(
                "missed resources updates: {:?}",
                resources_update.dropped_events
            );
        }

        if let Some(app) = self.database.applications_read().await.get(&app_id) {
            if app.state() == ApplicationState::Disabled {
                return;
            }

            // 1. Insert new resources
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

            // 2. Update existing resources
            for (id, updated_resource) in resources_update.stats_update {
                let mut resource_guard = self.database.resources_write().await;
                let key = format!("{}.{}", app.title(), id);
                if let Some(resource_arc) = resource_guard.get_mut(&key) {
                    let resource = Arc::make_mut(resource_arc);

                    // Handle dropped vs. running durations
                    if let Some(dropped_at) = updated_resource.dropped_at {
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
                                    nano = 1_000_000_000 + nano;
                                }
                                Some(Duration::new(seconds, nano))
                            };
                        }
                    }

                    // Build a multi-line attributes string
                    let mut attribute_str = String::from("");
                    for attr in updated_resource.attributes {
                        if let Some(field) = attr.field {
                            if let Some(name) = field.name {
                                // Field name
                                match name {
                                    //if the name is a String, we concatenate directly
                                    console_api::field::Name::StrName(s) => {
                                        attribute_str = attribute_str + &s;
                                    }

                                    //if attribute name is an index from metadata.field_names
                                    console_api::field::Name::NameIdx(_) => {
                                        // TODO: Handle NameIdx variants
                                    }
                                }
                                // Field value
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

            // 3. Append new poll operations
            for raw in resources_update.new_poll_ops {
                if let Some(mut domain_poll) = map_to_domain_poll(&raw) {
                    domain_poll.app_name = Some(app.title().to_string());
                    domain_poll.received_at = received_at.clone();

                    // Enrich from resource metadata
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

                    // Enrich from task metadata
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

    /// Returns a list of all resources currently stored in memory.
    ///
    /// Each entry is an `Arc<Resource>` representing the latest known
    /// state of that resource.
    pub(crate) async fn get_resources(&self) -> Vec<Arc<Resource>> {
        self.database
            .resources_read()
            .await
            .values()
            .cloned()
            .collect()
    }

    /// Returns the sequence of recorded `Poll` events.
    ///
    /// Each `Poll` is wrapped in an `Arc`. The returned vector preserves
    /// insertion order.
    pub(crate) async fn get_polls(&self) -> Vec<Arc<Poll>> {
        self.database.polls_read().await
    }

    // endregion

    // region Async op

    /// Processes a batch of asynchronous‐operation updates for the specified application.
    ///
    /// - `app_id`: UUID of the application emitting the async‐op events.
    /// - `async_op_update`: An `AsyncOpUpdate` containing:
    ///     • `new_async_ops`: newly discovered async operations to insert  
    ///     • `stats_update`: polling statistics for existing async operations  
    ///
    /// This method will:
    /// 1. Log and ignore any dropped event counts.  
    /// 2. If the application is disabled, skip all processing.  
    /// 3. For each new async operation:
    ///    – Map it into a domain `TaskOp` record.  
    ///    – Look up its resource by ID in the resource store; if found, record the resource target.  
    ///    – Insert the new `TaskOp` into the async‐ops store under the key `{app_title}.{op_id}`.  
    /// 4. For each stats update (keyed by async‐op ID):
    ///    – Locate the existing `TaskOp` using `{app_title}.{op_id}`.  
    ///    – If it exists and contains a matching in‐progress CPU poll entry,
    ///      update its `stopped_at` timestamp when the poll ends.  
    ///    – Otherwise, append a new `CPUOverview` entry (with `started_at` and optional `stopped_at`).  
    ///    – If no `TaskOp` record exists, create one from scratch using any known
    ///      task metadata (name/color) and the new CPU overview.  
    pub async fn handle_async_op_update(&self, app_id: Uuid, async_op_update: AsyncOpUpdate) {
        if async_op_update.dropped_events > 0 {
            println!(
                "missed async_op updates: {:?}",
                async_op_update.dropped_events
            );
        }

        // Skip processing if the app is disabled or missing
        if let Some(app) = self.database.applications_read().await.get(&app_id) {
            if app.state() == ApplicationState::Disabled {
                return;
            }

            // Insert any new async-ops
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

                // Attempt to fetch the existing entry and its resource target
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

    /// Retrieves all recorded asynchronous‐operation logs (CPU overviews).
    ///
    /// Returns a vector of `Arc<TaskOp>`, each containing the task’s
    /// ID, optional name and color, and the sequence of `CPUOverview`
    /// entries representing its polling history.
    pub(crate) async fn get_tasks_ops(&self) -> Vec<Arc<TaskOp>> {
        self.database
            .tasks_ops_read()
            .await
            .values()
            .cloned()
            .collect()
    }

    // endregion
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use url::Url;

    async fn make_state() -> State {
        let dir = tempdir().unwrap();
        let path = dir.path().to_string_lossy().to_string();
        let db = Database::new(path);
        State {
            database: Arc::new(db),
        }
    }

    #[tokio::test]
    async fn test_handle_and_get_pid() {
        let state = make_state().await;

        let app = Application::new_mock(
            "TestApp".into(),
            Url::parse("http://localhost").unwrap(),
            1234,
        );
        let id = *app.id();

        let app_arc = Arc::new(app);
        state
            .database
            .applications_write()
            .await
            .insert(id, Arc::clone(&app_arc));

        assert_eq!(state.get_pid_for(id).await, Some(1234));

        state.handle_pid_changed(id, 5678).await;
        assert_eq!(state.get_pid_for(id).await, Some(5678));
    }
}
