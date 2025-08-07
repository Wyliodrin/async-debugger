//! Connection manager for remote applications.
//!
//! This module maintains a set of active connections to instrumented applications.
//! It handles the lifecycle of each connection (connect, receive updates, refresh
//! process stats, disconnect), and multiplexes events back onto a channel that
//! can be observed by the rest of the system.

#![allow(unused)]

use crate::common::get_pid_hosting_at;
use crate::{
    domain::application::{self, Application},
    error::Error as TraceError,
};
use console_api::instrument::{instrument_client::InstrumentClient, InstrumentRequest, Update};
use log::{debug, error, info, warn};
use std::{clone, collections::HashMap, error::Error, sync::Arc, time::Duration};
use sysinfo::{Pid, ProcessRefreshKind, ProcessStatus, ProcessesToUpdate, System};
use tauri::Url;
use tokio::{
    select,
    sync::{
        mpsc::{self, Sender},
        RwLock,
    },
    time::{interval_at, sleep, Instant, Interval},
};
use tonic::{transport::Endpoint, Streaming};
use uuid::Uuid;

/// Commands you can send to an active connection task.
#[derive(Debug)]
pub enum Command {
    /// Instructs the connection task to disconnect and shut down.
    Disconnect,
}

/// Events emitted by the connection manager for each application.
#[non_exhaustive]
pub enum Event {
    /// Connection is in progress.
    Connecting,
    /// Successfully connected and ready to receive updates.
    Connected,
    /// An update pushed from the remote instrumented application.
    Update(Update),
    /// Periodic local process statistics (CPU, memory, status).
    ApplicationUpdated(AppUpdate),
    /// An error occurred during connection or streaming.
    Error(TraceError),
    /// The connection has been shut down.
    Disconnected,
}

/// Snapshot of local process statistics for an instrumented application.
#[derive(Clone)]
pub struct AppUpdate {
    /// CPU usage per core, as a fraction (0.0 – 100.0).
    pub cpu_usage: Option<f32>,
    /// Memory usage in megabytes.
    pub memory_usage: u64,
    /// Current OS process status.
    pub process_status: ProcessStatus,
}

/// A handle to an active connection.  You can send a [`Command::Disconnect`] on
/// the `commands` channel to terminate the connection.
#[derive(Clone, Debug)]
pub struct Connection {
    /// Channel on which to send connection commands.
    pub commands: Sender<Command>,
}

impl Drop for Connection {
    fn drop(&mut self) {
        debug!("Dropped connection");
    }
}

/// Manages multiple live connections to instrumented applications.  Spawns a
/// Tokio task for each connection to handle streaming updates and periodic
/// process stats.
pub struct ConnectionManager {
    /// Internal sender
    sender: Sender<(Uuid, Event)>,
    /// Map of active connection tasks, keyed by application UUID.
    active_connections: Arc<RwLock<HashMap<Uuid, tokio::task::JoinHandle<()>>>>,
}

impl Clone for Event {
    fn clone(&self) -> Self {
        match self {
            Event::Connecting => Event::Connecting,
            Event::Connected => Event::Connected,
            Event::Update(u) => Event::Update(u.clone()),
            Event::ApplicationUpdated(a) => Event::ApplicationUpdated(a.clone()),
            Event::Error(_) => panic!("Cannot clone Event::Error (rich TraceError)"),
            Event::Disconnected => Event::Disconnected,
        }
    }
}

impl ConnectionManager {
    /// Create a new `ConnectionManager`.
    ///
    /// # Parameters
    /// - `tx`: the channel to send `(Uuid, Event)` tuples on.
    ///
    /// # Returns
    /// A fresh manager without any active connections.
    pub fn new(sender: Sender<(Uuid, Event)>) -> Self {
        Self {
            sender,
            active_connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Connect to an instrumented application.
    ///
    /// Spawns a background Tokio task that:
    /// - Signals `Event::Connecting`
    /// - Attempts to establish a gRPC stream to the `url`
    /// - Emits `Event::Connected` then forwards all incoming `Update` messages
    ///   as `Event::Update`
    /// - Every second, polls local process info (CPU, memory, status) and emits
    ///   `Event::ApplicationUpdated`
    /// - Retries on failure until a `Command::Disconnect` is received
    ///
    /// # Parameters
    /// - `id`: unique application identifier (used as the map key)
    /// - `url`: the gRPC endpoint of the instrumenter
    /// - `pid`: the OS process ID of the instrumented application
    ///
    /// # Errors
    /// Returns an error if the application is already connected.
    pub async fn connect_app(
        &self,
        id: Uuid,
        url: Url,
        mut pid: u32,
    ) -> Result<Connection, TraceError> {
        // Create command channel for this connection
        let (command_sender, mut command_receiver) = mpsc::channel(100);
        let connection = Connection {
            commands: command_sender,
        };
        let updates_sender = self.sender.clone();

        // Check if app already connected
        if self.active_connections.read().await.contains_key(&id) {
            warn!("Tried to add application with uuid {}, but the id is already attached to a connected application", id);
            return Err(TraceError::ApplicationAlreadyConnected(id.to_string()));
        }

        // Spawn the background task
        let cloned_id = id.clone();
        let connection_task = tokio::task::spawn(async move {
            let mut sys = sysinfo::System::new_all();
            sys.refresh_all();

            'connection: loop {
                // Notify that we're starting a connection attempt
                updates_sender
                    .send((cloned_id, Event::Connecting))
                    .await
                    .ok();

                info!("Connecting to application with url {}", url);

                // Connect the app
                let connection = 'connect: loop {
                    select! {
                        connection = Self::connect_to_app(&url) => {
                            // m-am conectat, astept comenzi mai jos
                            debug!("Received connection result");
                            break 'connect connection;
                        }
                        command = command_receiver.recv() => {
                            debug!("Received command: {:?}", command);
                            match command {
                                Some(Command::Disconnect) | None => break 'connection
                            }
                        }
                    };
                };

                // Vad daca primesc comenzi pt aplicatie (gen disconnect/disable)
                // Check connection
                match connection {
                    Ok(mut update_stream) => {
                        info!("Successfully connected to application with url {}", url);

                        updates_sender
                            .send((cloned_id, Event::Connected))
                            .await
                            .ok();

                        let mut refresh = interval_at(Instant::now(), Duration::from_secs(1));

                        // Main loop: handle incoming updates, commands, or tick
                        loop {
                            select! {
                                // Wait for new updates regarding our app
                                update = update_stream.message() => {
                                    debug!("Received task update");
                                    match update {
                                        Ok(message) => {
                                            if let Some(update) = message {
                                                info!("Received an update about application with url {}", url);
                                                updates_sender.send((cloned_id, Event::Update(update))).await.ok();
                                    }
                                        }
                                        Err(_error) => {
                                            // TODO report error
                                            // for now we disconnect
                                        continue 'connection;
                                    }
                                    }
                                }
                                // Wait for external commands
                                command = command_receiver.recv() => {
                                    debug!("Received command");
                                    if let Some(command) = command {
                                        match command {
                                            Command::Disconnect => break 'connection,
                                        }
                                    } else {
                                        // Command stream is closed so we exit
                                        break 'connection;
                                    }
                                }
                                // Should refresh data stored about app
                                // TODO TEST: cgecj if we receive the app updates once per second
                                _ = refresh.tick() => {
                                    debug!("Sending application info refresh");
                                    if let Some(app_update)= {
                                    sys.refresh_all();
                                         // Wait a bit because CPU usage is based on diff.
                                        tokio::time::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL).await;
                                        // Refresh CPU usage to get actual value.
                                    sys.refresh_processes_specifics(
                                        ProcessesToUpdate::All,
                                        true,
                                        ProcessRefreshKind::nothing().with_cpu(),
                                    );
                                        Self::check_app_stats(&mut sys, pid).await
                                        } {
                                        updates_sender.send((cloned_id, Event::ApplicationUpdated(app_update))).await.ok();
                                    } else {
                                        if let Some(new_pid) = get_pid_hosting_at(url.clone()){
                                            if new_pid != pid {
                                                pid = new_pid;
                                            }
                                            else {
                                                updates_sender.send((cloned_id, Event::Error(TraceError::CannotReadProcessInfo { pid }))).await.ok();
                                            }
                                        }
                                        else {
                                            updates_sender.send((cloned_id, Event::Error(TraceError::CannotReadProcessInfo { pid }))).await.ok();
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(error) => {
                        error!(
                            "Could not connect to application with url {} due to {error:?}",
                            url
                        );
                        updates_sender
                            .send((cloned_id, Event::Error(TraceError::Anyhow(error.into()))))
                            .await
                            .ok();

                        // Sleep before trying to connect again
                        sleep(Duration::from_secs(1)).await;
                    }
                }
            }

            // Final notification of disconnection
            updates_sender
                .send((cloned_id, Event::Disconnected))
                .await
                .ok();
        });

        // Store the task handle
        self.active_connections
            .write()
            .await
            .insert(id, connection_task);
        return Ok(connection);
    }

    /// Forcefully remove the connection task for the given `uuid`.
    /// This will drop its `Connection` handle and stop receiving further events.
    pub(crate) async fn disconnect_app(&self, uuid: Uuid) {
        self.active_connections.write().await.remove(&uuid);
    }

    /// Internal helper: open a gRPC streaming connection to the remote instrumenter.
    ///
    /// Returns a boxed `tonic::Streaming<Update>` on success.
    async fn connect_to_app(url: &Url) -> Result<Box<Streaming<Update>>, TraceError> {
        let endpoint = Endpoint::new(url.to_string()).map_err(|e| TraceError::Anyhow(e.into()))?;
        debug!("Created the endpoint");
        let channel =
            endpoint
                .connect()
                .await
                .map_err(|e| TraceError::CannotCreateChannelForApp {
                    url: url.to_string(),
                })?;
        debug!("Created channel");

        let mut client = InstrumentClient::new(channel);
        let update_request = tonic::Request::new(InstrumentRequest {});

        let stream = client
            .watch_updates(update_request)
            .await
            .map_err(|e| TraceError::Anyhow(e.into()))?
            .into_inner();

        debug!("Obtained updates stream");

        Ok(Box::new(stream))
    }

    /// Internal helper: read CPU, memory and status for the given `pid` from `sys`.
    ///
    /// Returns `None` if the process no longer exists.
    async fn check_app_stats(sys: &mut System, pid: u32) -> Option<AppUpdate> {
        let cpu_count = sys.cpus().len() as f32;
        // println!("CPUS: {cpu_count}");
        let process = sys.process(Pid::from_u32(pid))?;
        let cpu_per_core = process.cpu_usage() / cpu_count;
        let memory_mb = process.memory() / 1000000;

        Some(AppUpdate {
            cpu_usage: Some(cpu_per_core),
            memory_usage: memory_mb,
            process_status: process.status(),
        })
    }
}
