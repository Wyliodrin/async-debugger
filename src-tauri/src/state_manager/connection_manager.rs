use crate::error::Error as TraceError;
use console_api::instrument::{instrument_client::InstrumentClient, InstrumentRequest, Update};
use log::{error, info};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tauri::Url;
use tokio::{
    select,
    sync::mpsc::{self, Sender},
    sync::RwLock,
};
use tonic::{transport::Endpoint, Streaming};
use uuid::Uuid;

pub enum Command {
    Disconnect,
}

#[non_exhaustive]
pub enum Event {
    Connecting,
    Connected,
    Update(Update),
    Error(TraceError),
    Disconnected,
}

// TODO: need to check if still needed
#[derive(Clone, Debug)]
pub struct Connection {
    pub commands: Sender<Command>,
}

pub struct ConnectionManager {
    updates_sender: Sender<(Uuid, Event)>,
    active_connections: Arc<RwLock<HashMap<Uuid, tokio::task::JoinHandle<()>>>>,
}

impl ConnectionManager {
    pub fn new(updates_sender: Sender<(Uuid, Event)>) -> Self {
        Self {
            updates_sender,
            active_connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    pub async fn connect_app(&self, uuid: Uuid, url: Url) -> Result<Connection, TraceError> {
        let (command_sender, mut command_receiver) = mpsc::channel(100);
        let connection = Connection {
            commands: command_sender,
        };

        let updates_sender = self.updates_sender.clone();

        if self.active_connections.read().await.contains_key(&uuid) {
            return Err(TraceError::ApplicationAlreadyConnected(uuid));
        }

        let connection_task = tokio::task::spawn(async move {
            'connection: loop {
                // TODO: de verificat cine asculta, eventual in UI ca sa afisam ca suntem sau nu conectati la aplicatie
                updates_sender.send((uuid, Event::Connecting)).await.ok();

                // Incerc sa ma conectez
                let connection = 'connect: loop {
                    select! {
                        connection = Self::connect_to_app(&url) => {
                            // m-am conectat, astept comenzi mai jos
                            break 'connect connection;
                        }
                        command = command_receiver.recv() => {
                            match command {
                                Some(Command::Disconnect) | None => break 'connection
                            }
                        }
                    };
                };

                // Vad daca primesc comenzi pt aplicatie (gen disconnect/disable)
                match connection {
                    Ok(mut update_stream) => {
                        info!("Successfully connected to application with url {url}");

                        // todo: cine asculta aici?
                        updates_sender.send((uuid, Event::Connected)).await.ok();

                        loop {
                            select! {
                                update = update_stream.message() => {
                                    match update {
                                        Ok(message) => {
                                            if let Some(update) = message {
                                                info!("Received an update about application with url {url}");
                                                updates_sender.send((uuid, Event::Update(update))).await.ok();
                                            }
                                        }
                                        Err(_error) => {
                                            // TODO report error
                                            // for now we disconnect
                                            continue 'connection;
                                        }
                                    }
                                }
                                command = command_receiver.recv() => {
                                    if let Some(command) = command {
                                        match command {
                                            Command::Disconnect => break 'connection,
                                        }
                                    } else {
                                        // command stream is closed,
                                        // disconnect
                                        break 'connection;
                                    }
                                }
                            }
                        }
                    }
                    Err(error) => {
                        error!("Could not connect to application with url {url} due to {error:?}");
                        updates_sender
                            .send((uuid, Event::Error(TraceError::Anyhow(error.into()))))
                            .await
                            .ok();
                        tokio::time::sleep(Duration::from_secs(5)).await;
                    }
                }
            }

            updates_sender.send((uuid, Event::Disconnected)).await.ok();
        });

        self.active_connections
            .write()
            .await
            .insert(uuid, connection_task);
        return Ok(connection);
    }

    pub(crate) async fn disconnect_app(&self, uuid: Uuid) {
        self.active_connections.write().await.remove(&uuid);
    }

    async fn connect_to_app(url: &Url) -> Result<Box<Streaming<Update>>, TraceError> {
        let endpoint = Endpoint::new(url.to_string()).map_err(|e| TraceError::Anyhow(e.into()))?;
        let channel = endpoint
            .connect()
            .await
            .map_err(|e| TraceError::Anyhow(e.into()))?;
        let mut client = InstrumentClient::new(channel);
        let update_request = tonic::Request::new(InstrumentRequest {});
        Ok(Box::new(
            client
                .watch_updates(update_request)
                .await
                .map_err(|e| TraceError::Anyhow(e.into()))?
                .into_inner(),
        ))
    }
}
