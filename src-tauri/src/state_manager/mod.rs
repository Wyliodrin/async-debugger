// TODO: check if pub needed
pub mod connection_manager;
mod database;
pub mod state;

use crate::domain::application::Application;
use crate::error::Error as TraceError;
use crate::state_manager::state::State;
use anyhow::Result;
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
    pub async fn new() -> Result<(StateManager, Receiver<(Uuid, Event)>), TraceError> {
        let (updates_sender, updates_receiver) = mpsc::channel(100);

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

        let connection_manager = ConnectionManager::new(updates_sender);

        let context = StateManager {
            connection_manager,
            state,
        };

        Ok((context, updates_receiver))
    }

    // region events

    pub async fn run(&self, mut updates_receiver: Receiver<(Uuid, Event)>) {
        self.reconnect_all_apps().await;

        // event loop
        loop {
            tokio::select! {
                // Received updates about apps
                Some((app_id, event)) = updates_receiver.recv() => {
                    match event {
                        Event::TaskUpdate(update) => {
                            if let Some(task_update) = update.task_update {
                                self.state.handle_task_update(app_id, task_update).await;
                            }
                        }
                        Event::ApplicationUpdated(update) => {
                            self.state.handle_app_update(app_id, update).await;
                        }
                        _ => {}
                    }
                },

                // todo: add other events receivers
                // todo: add receiver to add application and send to connection manager then update state
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

        // TODO: enable app from state

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

    /// Returns a list of the applications currently registered in the app
    /// (not necessarily active too)
    pub async fn _current_applications(&self) -> Vec<Arc<Application>> {
        self.state.get_current_applications_list().await
    }

    // pub async fn delete_connection(&self, uuid: Uuid) {
    //     self.connection_manager.disconnect_app(uuid).await;
    //     self.state.delete_app(uuid).await
    // }

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

    // endregion
}
