// TODO: check if pub needed
pub mod connection_manager;
mod database;
pub mod state;

use crate::domain::application::{Application, ConnectionStatus};
use crate::error::Error as TraceError;
use crate::infra::spy_channel::{SpyEvent, SpySender};
use crate::state_manager::connection_manager::Connection;
use crate::state_manager::state::State;
use anyhow::Result;
use chrono::{DateTime, Local, TimeZone};
use connection_manager::{ConnectionManager, Event};
use log::{debug, error, info};
use std::sync::Arc;
use tauri::{AppHandle, Emitter as _};
use tokio::sync::mpsc::{self, Receiver};
use url::Url;
use uuid::Uuid;

pub struct StateManager {
    // Mpsc used to receive updates about connected applications
    // (eg. number of running tasks, time ran)
    // TODO: check if needed
    // pub updates_sender: Sender<(Uuid, Event)>,

    // Manages the connection to the running applications
    // and sends updates about them
    pub connection_manager: ConnectionManager,

    pub state: State,
}

impl StateManager {
    pub async fn new() -> Result<
        (
            StateManager,
            Receiver<(Uuid, Event)>,
            Receiver<(Uuid, SpyEvent)>,
        ),
        TraceError,
    > {
        // TODO: check if error handling could be done better here (maybe looking for a single error is not the best case)
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

        let (real_tx, real_rx) = mpsc::channel::<(Uuid, Event)>(100);
        let (spy_tx, spy_rx) = mpsc::channel::<(Uuid, SpyEvent)>(100);
        let _spy_sender = SpySender::new(real_tx.clone(), spy_tx.clone());
        let connection_manager = ConnectionManager::new(real_tx.clone(), spy_tx.clone());

        let context = StateManager {
            connection_manager,
            state,
        };

        Ok((context, real_rx, spy_rx))
    }

    // region events

    pub async fn run(&self, mut updates_receiver: Receiver<(Uuid, Event)>) {
        self.reconnect_all_apps().await;
        let mut skip_first_update = true;

        // event loop
        loop {
            tokio::select! {
                // Received updates about apps
                Some((app_id, event)) = updates_receiver.recv() => {
                    match event {
                        Event::Update(update) => {
                            let task_future = async {
                                if let Some(task_update) = update.task_update {
                                    self.state.handle_task_update(app_id, task_update).await;
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
                                            Some(pretty)
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
                    }
                }

                // TODO: add other events receivers
                // TODO: add receiver to add application and send to connection manager then update state
            }
        }
    }

    // endregion

    // region application

    /// Registers and enables a new application
    ///
    /// Is also connecting to the application in order to receive updates about it
    pub async fn add_application(&self, title: String, url: Url) -> Result<Uuid, TraceError> {
        // Create and enable application
        let mut application = Application::new(title, url)?;
        let app_id = application.id().clone();

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

    // pub async fn enable_application(&self, uuid: Uuid) -> Result<(), TraceError> {
    // self.state.enable_app(uuid, connection).await
    // }

    pub async fn reconnect_all_apps(&self) {
        let apps_list = self.state.get_current_applications_list().await;
        for app in apps_list {
            // TODO: check if we should retry in case of error
            info!(
                "Reconnecting application {} that has PID {}",
                app.title(),
                app.pid()
            );
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

    pub async fn disable_application(&self, uuid: Uuid) -> Result<(), TraceError> {
        self.state.disable_app(uuid).await
    }

    pub async fn delete_application(&self, uuid: Uuid) -> Result<Uuid, TraceError> {
        self.state.delete_application(uuid).await;
        Ok(uuid)
    }

    pub async fn _enable_application(&self, uuid: Uuid, connection: Connection) {
        self.state.enable_app(uuid, connection).await
    }

    /// Returns a list of the applications currently registered in the app
    /// (not necessarily active too)
    pub async fn current_applications(&self) -> Vec<Arc<Application>> {
        self.state.get_current_applications_list().await
    }

    pub async fn delete_connection(&self, uuid: Uuid) {
        self.connection_manager.disconnect_app(uuid).await;
        //     self.state.delete_app(uuid).await
    }

    // endregion

    // region UPDATES

    pub async fn emit_update_tasks(&self, app_handle: &AppHandle) {
        let tasks = self.state.get_tasks().await;
        app_handle.emit("update:tasks", tasks).ok();
    }

    pub async fn emit_update_applications(&self, app_handle: &AppHandle) {
        let elements = self.state.get_current_applications_list().await;
        app_handle.emit("update:applications", elements).ok();
    }

    pub async fn emit_update_resources(&self, app_handle: &AppHandle) {
        let resources = self.state.get_resources().await;
        app_handle.emit("update:resources", resources).ok();
    }

    pub async fn emit_update_polls(&self, app_handle: &AppHandle) {
        let polls = self.state.get_polls().await;
        app_handle.emit("update:polls", polls).ok();
    }

    pub async fn emit_update_tasks_op(&self, app_handle: &AppHandle) {
        let tasks_op = self.state.get_tasks_ops().await;
        app_handle.emit("update:tasks_ops", tasks_op).ok();
    }

    // pub async fn emit_connection_update(&self, app_id: )
    // endregion
}
