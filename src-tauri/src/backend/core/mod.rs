//! State manager for applications, tasks, resources, and their connections.
//!
//! This module ties together three main components:
//! 1. `State` – in‐memory and persistent application state (via `state_manager::state`).  
//! 2. `ConnectionManager` – background gRPC streams to instrumented applications.  
//! 3. Tauri event emitters – send updated data to the frontend UI.  
//!
//! The `StateManager` orchestrates loading previous state, reconnecting known
//! applications on startup, handling incoming gRPC events, updating the domain
//! state, and emitting frontend events when requested.

pub mod connection_manager;
mod database;
pub mod state;
pub mod warnings;

use crate::backend::core::connection_manager::Connection;
use crate::backend::core::state::State;
use crate::backend::domain::application::{Application, ConnectionStatus};
use crate::features::applications::{ExportEntry, TimestampEntry};
use crate::utils::error::Error as TraceError;
use anyhow::Result;
use chrono::{DateTime, Local, TimeZone};
use connection_manager::{ConnectionManager, Event};
use log::{debug, error, info};
use std::path::Path;
use std::sync::Arc;
use tauri::{AppHandle, Emitter as _};
use tokio::fs;
use tokio::sync::mpsc::{self, Receiver};
use url::Url;
use uuid::Uuid;

/// Top-level orchestrator for application state and connections.
///
/// - Loads or initializes the persistent `State`.  
/// - Spawns a `ConnectionManager` to handle gRPC streams to instrumented apps.  
/// - Listens for update events and dispatches them into the `State`.  
/// - Provides methods to add, enable/disable, delete, and list applications.  
/// - Emits Tauri events containing the latest tasks, resources, polls, and apps.
pub struct StateManager {
    /// Handles all gRPC connections and streams of events.
    pub connection_manager: ConnectionManager,

    /// In-memory and persistent domain state.
    pub state: State,
}

impl StateManager {
    /// Initialize a new `StateManager`.
    ///
    /// Attempts to load the previous `State` from disk. If loading fails with
    /// a non‐recoverable error (`CannotCreateStorage`), returns an error. On
    /// any other load failure, logs and falls back to a fresh `State::new()`.
    ///
    /// Also creates a channel:
    /// - `Receiver<(Uuid, Event)>` for application events  
    ///
    /// # Returns
    /// `(StateManager, rx)
    pub async fn new() -> Result<(StateManager, Receiver<(Uuid, Event)>), TraceError> {
        // Load or initialize the persisted state
        let state = match State::load().await {
            // State loaded successfully
            Ok(state) => state,
            Err(error) => {
                match error {
                    // Could not create storage location,
                    TraceError::CannotCreateStorage { error, path } => {
                        return Err(TraceError::CannotCreateStorage { error, path })
                    }
                    // For any other errors we use a fresh state
                    err => {
                        error!("Failed to load previous state due to {err:?}. Using new State instance");
                        State::new()
                    }
                }
            }
        };

        // Create channel for events
        let (tx, rx) = mpsc::channel::<(Uuid, Event)>(100);
        // Initialize the connection manager
        let connection_manager = ConnectionManager::new(tx.clone());

        let context = StateManager {
            connection_manager,
            state,
        };

        Ok((context, rx))
    }

    /// Main event loop.
    ///
    /// - Reconnects all applications known in `State` at startup.  
    /// - Waits for `(Uuid, Event)` messages from the `ConnectionManager`.  
    /// - On each event, dispatches to the appropriate `State` handler:
    ///   - `Event::Update` → task / resource / async_op updates  
    ///   - `Event::ApplicationUpdated` → process stats  
    ///   - `Event::Connecting` / `Connected` / `Disconnected` / `Error` → connection status  
    pub async fn run(&self, mut updates_receiver: Receiver<(Uuid, Event)>) {
        self.reconnect_all_apps().await;
        let mut skip_first_update = true;

        loop {
            tokio::select! {
            // Received updates about apps
            Some((app_id, event)) = updates_receiver.recv() => {
                match event {
                    Event::Update(update) => {
                        let mut warnings = Vec::new();
                        let task_future = async {
                            if let Some(task_update) = update.task_update {
                                warnings.extend(self.state.handle_task_update(app_id, task_update).await);
                            }
                        };

                        let resource_future = async {
                            if let Some(resource_update) = update.resource_update {
                                let update_time = {
                                    if update.now.is_some() {
                                        let received_update_time = update.now
                                            .as_ref()
                                            .expect("we just tested is_some()");
                                        let dt_local: DateTime<Local> = Local
                                            .timestamp_opt(received_update_time.seconds, received_update_time.nanos as u32)
                                            .single()
                                            .expect("timestamp invalid");
                                        Some(dt_local)
                                    } else {
                                        None
                                    }
                                };
                                self.state.handle_resource_update(app_id, resource_update, update_time).await;
                            }
                        };

                        let async_op_future = async {
                            if let Some(async_op_update) = update.async_op_update{
                                if skip_first_update {
                                    skip_first_update = false;
                                }
                                else{
                                    self.state.handle_async_op_update(app_id, async_op_update).await;
                                }
                            }
                        };

                        tokio::join!(task_future, resource_future, async_op_future);
                        println!("{:?}", warnings);
                    },

                    Event::ApplicationUpdated(update) => {
                        self.state.handle_app_update(app_id, update).await;
                    },

                    Event::Connecting => {
                        println!("Connecting..");
                        self.state.handle_app_conn_update(app_id, ConnectionStatus::Connecting).await;
                    },

                    Event::Connected => {
                        println!("Connected");
                        self.state.handle_app_conn_update(app_id, ConnectionStatus::Connected).await;
                    },

                    Event::Disconnected => {
                        println!("Disconnected app");
                        self.delete_connection(app_id).await;
                        self.state.handle_app_conn_update(app_id, ConnectionStatus::Disconnected).await;
                    },

                    Event::Error(err) => {
                        println!("Error with app connection: {err:?}");
                        self.state.handle_app_conn_update(app_id, ConnectionStatus::Error(err.to_string())).await;
                    }

                    Event::PidChanged(new_pid) => {
                        println!("PID changed to {new_pid}");
                        self.state.handle_pid_changed(app_id, new_pid).await;
                    }
                }
            }

                // TODO: add other events receivers
                // TODO: add receiver to add application and send to connection manager then update state
                }
        }
    }

    //--------------------------------------------------------------------------
    // Application management
    //--------------------------------------------------------------------------

    /// Create, persist, and connect a new application.
    ///
    /// - Constructs `Application::new(title, url)?`.  
    /// - Spawns a gRPC connection via `ConnectionManager::connect_app`.  
    /// - Marks the app as enabled and stores it in `State`.  
    ///
    /// Is also connecting to the application in order to receive updates about it
    pub async fn add_application(&self, title: String, url: Url) -> Result<Uuid, TraceError> {
        // Create and enable application
        let mut application = Application::new(title, url)?;
        let app_id = *application.id();

        // Connect to the app
        let connection = self
            .connection_manager
            .connect_app(*application.id(), application.url(), application.pid())
            .await?;
        application.enable(connection);

        // Store app
        self.state.store_app(application).await;

        Ok(app_id)
    }

    /// Reconnect all previously registered applications at startup.
    ///
    /// For each app in `State`, calls `connect_app`. On success, marks it
    /// enabled; on failure, logs an error.
    pub async fn reconnect_all_apps(&self) {
        let apps_list = self.state.get_current_applications_list().await;
        for app in apps_list {
            info!("Reconnecting {} (PID {})", app.title(), app.pid());
            let connection_result = self
                .connection_manager
                .connect_app(*app.id(), app.url().clone(), app.pid())
                .await
                .map_err(|err| {
                    error!("Failed to reconnect app at startup {:?}", err);
                    err
                });

            if let Ok(connection) = connection_result {
                debug!("Enabling app {}", app.title());
                self.state.enable_app(*app.id(), connection).await;
            } else {
                // TODO
                todo!();
            }
        }
    }

    /// Disable an application’s updates without deleting its record.
    pub async fn disable_application(&self, uuid: Uuid) -> Result<(), TraceError> {
        self.state.disable_app(uuid).await
    }

    /// Edits an existing application identified by its UUID.
    ///
    /// This asynchronous function performs the following steps:
    /// 1. Deletes the application with the specified UUID.
    /// 2. Renames the application key from `old_title` to `app_title` in the internal state.
    /// 3. Parses the new application URL and adds the updated application with the new title and URL.
    ///
    /// # Parameters
    ///
    /// * `uuid` - The unique identifier of the application to be edited.
    /// * `app_title` - The new title for the application.
    /// * `app_url` - The new URL associated with the application.
    /// * `old_title` - The old title of the application to be replaced.
    ///
    /// # Returns
    ///
    /// Returns a `Result` which is:
    /// - `Ok(Uuid)` with the UUID of the updated application if successful.
    /// - `Err(TraceError)` if any step fails, including deletion, renaming, URL parsing, or addition.
    pub async fn edit_application(
        &self,
        uuid: Uuid,
        app_title: String,
        app_url: String,
        old_title: String,
    ) -> Result<Uuid, TraceError> {
        self.delete_application(uuid).await?;
        self.state.edit_app(app_title.clone(), old_title).await?;
        let url = Url::parse(&app_url)?;
        self.add_application(app_title.clone(), url).await
    }

    /// Delete an application from state.
    ///
    /// Disconnects it (if enabled), removes its folder on disk, and
    /// deletes it from the in-memory `State`.
    pub async fn delete_application(&self, uuid: Uuid) -> Result<Uuid, TraceError> {
        self.state.delete_application(uuid).await;
        Ok(uuid)
    }

    pub async fn _enable_application(&self, uuid: Uuid, connection: Connection) {
        self.state.enable_app(uuid, connection).await
    }
    /// List all applications (enabled or not).
    pub(crate) async fn current_applications(&self) -> Vec<Arc<Application>> {
        self.state.get_current_applications_list().await
    }

    pub async fn delete_connection(&self, uuid: Uuid) {
        self.connection_manager.disconnect_app(uuid).await;
        //     self.state.delete_app(uuid).await
    }

    /// Emit the latest PID for `app_id` to the frontend.
    ///
    /// Listeners should handle the `"update:pid"` event.
    pub async fn emit_update_pid(&self, app_handle: &AppHandle, app_id: Uuid) {
        // Fetch the latest PID from State
        let maybe_pid = {
            let apps = self.state.get_current_applications_list().await;
            apps.into_iter()
                .find(|app| *app.id() == app_id)
                .map(|app| app.pid())
        };

        if let Some(pid) = maybe_pid {
            let payload = serde_json::json!({
                "id": app_id,
                "pid": pid,
            });
            app_handle.emit("update:pid", payload).ok();
        }
    }

    // endregion

    // region UPDATES
    //--------------------------------------------------------------------------
    // Frontend event emitters
    //--------------------------------------------------------------------------

    /// Emit the current tasks list to the Tauri front end.
    pub async fn emit_update_tasks(&self, app_handle: &AppHandle) {
        let tasks = self.state.get_tasks().await;
        app_handle.emit("update:tasks", tasks).ok();
    }

    /// Emit the current applications list to the Tauri front end.
    pub async fn emit_update_applications(&self, app_handle: &AppHandle) {
        let elements = self.state.get_current_applications_list().await;
        app_handle.emit("update:applications", elements).ok();
    }

    /// Emit the current resources list to the Tauri front end.
    pub async fn emit_update_resources(&self, app_handle: &AppHandle) {
        let resources = self.state.get_resources().await;
        app_handle.emit("update:resources", resources).ok();
    }

    /// Emit the current polls list to the Tauri front end.
    pub async fn emit_update_polls(&self, app_handle: &AppHandle) {
        let polls = self.state.get_polls().await;
        app_handle.emit("update:polls", polls).ok();
    }

    /// Emit the current task‐ops list to the Tauri front end.
    pub async fn emit_update_tasks_op(&self, app_handle: &AppHandle) {
        let tasks_op = self.state.get_tasks_ops().await;
        app_handle.emit("update:tasks_ops", tasks_op).ok();
    }

    /// Checks duplicates by url and title
    pub async fn ensure_not_connected(&self, title: &str, url: &str) -> Result<(), TraceError> {
        let applications = self.current_applications().await;

        for app in applications {
            if app.url().to_string() == url {
                return Err(TraceError::ApplicationAlreadyConnected(url.to_string()));
            }
            if app.title() == title {
                return Err(TraceError::ApplicationAlreadyConnected(title.to_string()));
            }
        }
        Ok(())
    }

    /// Validates duplicates, then parses the url and adds the app
    pub async fn add_application_if_absent(
        &self,
        title: String,
        url: &str,
    ) -> Result<uuid::Uuid, TraceError> {
        self.ensure_not_connected(&title, url).await?;
        let url: url::Url = url.try_into()?;
        self.add_application(title, url).await
    }

    /// enables flow: find app, connect via manager, mark it enabled
    pub async fn enable_app(&self, uuid: Uuid) -> Result<(), TraceError> {
        let apps = self.state.get_current_applications_list().await;
        let app = apps
            .iter()
            .find(|a| a.id() == &uuid)
            .ok_or_else(|| TraceError::Anyhow(anyhow::anyhow!("App {uuid} not found")))?;

        // ask the existing connection manager to connect.
        let conn: Connection = self
            .connection_manager
            .connect_app(*app.id(), app.url().clone(), app.pid())
            .await?;

        info!("enable_app: {uuid}");
        self.state.enable_app(uuid, conn).await;
        Ok(())
    }

    /// scans the given exports base and returns directories as ExportEntry, empty if missing
    pub async fn list_exports_from_base(
        &self,
        exports_base: &Path,
    ) -> Result<Vec<ExportEntry>, String> {
        let mut out: Vec<ExportEntry> = Vec::new();

        let mut dir = match fs::read_dir(exports_base).await {
            Ok(rd) => rd,
            Err(_) => return Ok(out),
        };

        while let Some(entry) = dir.next_entry().await.map_err(|e| e.to_string())? {
            let file_type = entry.file_type().await.map_err(|e| e.to_string())?;
            if file_type.is_dir() {
                let folder_name = entry.file_name().to_string_lossy().to_string();
                let title = folder_name.replace('-', " ");
                out.push(ExportEntry {
                    app_dir: folder_name,
                    title,
                });
            }
        }
        Ok(out)
    }

    pub async fn app_timestamps(
        &self,
        storage_folder: String,
        app_dir: String,
    ) -> Result<Vec<TimestampEntry>, String> {
        let app_folder = Path::new(&storage_folder).join("exports").join(&app_dir);
        let mut out: Vec<TimestampEntry> = Vec::new();

        let read_dir = match fs::read_dir(&app_folder).await {
            Ok(rd) => rd,
            Err(_) => return Ok(out),
        };

        let mut dir = read_dir;
        while let Some(entry) = dir.next_entry().await.map_err(|e| e.to_string())? {
            let meta = entry.file_type().await.map_err(|e| e.to_string())?;
            if meta.is_dir() {
                let ts_name = entry.file_name().to_string_lossy().to_string();
                let comment_path = app_folder.join(&ts_name).join("comment.txt");
                let comment_preview = match fs::read_to_string(&comment_path).await {
                    Ok(s) => {
                        let s = s.trim().to_string();
                        if s.is_empty() {
                            None
                        } else {
                            Some(if s.len() > 200 {
                                s[..200].to_string() + "..."
                            } else {
                                s
                            })
                        }
                    }
                    Err(_) => None,
                };
                out.push(TimestampEntry {
                    ts: ts_name.clone(),
                    path: app_folder.join(&ts_name).to_string_lossy().to_string(),
                    comment_preview,
                });
            }
        }

        out.sort_by(|a, b| b.ts.cmp(&a.ts));
        Ok(out)
    }

    pub async fn import_from_exp_folder(
        &self,
        storage_folder: String,
        app_dir: String,
        ts: String,
    ) -> Result<(), String> {
        let base = Path::new(&storage_folder)
            .join("exports")
            .join(&app_dir)
            .join(&ts);

        if !base.exists() {
            return Err(format!(
                "Export folder not found: {}",
                base.to_string_lossy()
            ));
        }
        self.state
            .import_from_export_folder(base)
            .await
            .map_err(|e| format!("Import error: {}", e))?;

        Ok(())
    }
}
