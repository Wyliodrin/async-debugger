#![allow(unused)]

use crate::common::get_pid_hosting_at;
use crate::{
    domain::application::{self, Application},
    error::Error as TraceError,
    infra::spy_channel::{SpyEvent, SpySender},
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

#[derive(Debug)]
pub enum Command {
    Disconnect,
}

#[non_exhaustive]
pub enum Event {
    Connecting,
    Connected,
    Update(Update),
    ApplicationUpdated(AppUpdate),
    Error(TraceError),
    Disconnected,
}

#[derive(Clone)]
pub struct AppUpdate {
    pub cpu_usage: Option<f32>,
    pub memory_usage: u64,
    pub process_status: ProcessStatus,
}

// TODO: need to check if still needed
#[derive(Clone, Debug)]
pub struct Connection {
    pub commands: Sender<Command>,
}

impl Drop for Connection {
    fn drop(&mut self) {
        debug!("Dropped connection");
    }
}

pub struct ConnectionManager {
    sender: SpySender,
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
    pub fn new(real_tx: Sender<(Uuid, Event)>, spy_tx: Sender<(Uuid, SpyEvent)>) -> Self {
        let sender = SpySender::new(real_tx, spy_tx);
        Self {
            sender,
            active_connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    pub async fn connect_app(
        &self,
        id: Uuid,
        url: Url,
        mut pid: u32,
    ) -> Result<Connection, TraceError> {
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

        let cloned_id = id.clone();
        let connection_task = tokio::task::spawn(async move {
            let mut sys = sysinfo::System::new_all();
            sys.refresh_all();
            'connection: loop {
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

                        // Wait for events
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
                                            else{
                                                updates_sender.send((cloned_id, Event::Error(TraceError::CannotReadProcessInfo { pid }))).await.ok();
                                            }
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

            updates_sender
                .send((cloned_id, Event::Disconnected))
                .await
                .ok();
        });

        self.active_connections
            .write()
            .await
            .insert(id, connection_task);
        return Ok(connection);
    }

    pub(crate) async fn disconnect_app(&self, uuid: Uuid) {
        self.active_connections.write().await.remove(&uuid);
    }

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
