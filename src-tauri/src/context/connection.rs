use std::{fmt, time::Duration};

use anyhow::{Error, Result};
use console_api::instrument::{instrument_client::InstrumentClient, InstrumentRequest, Update};
use tauri::{async_runtime, Url};
use tokio::{
    select,
    sync::mpsc::{self, Sender},
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
    Error(Error),
    Disconnected,
}

pub struct Connection {
    pub commands: Sender<Command>,
}

impl fmt::Debug for Connection {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // TODO display the capacity of the channels
        Ok(())
    }
}

async fn connect(url: &Url) -> Result<Box<Streaming<Update>>> {
    let endpoint = Endpoint::new(url.to_string())?;
    let channel = endpoint.connect().await?;
    let mut client = InstrumentClient::new(channel);
    let update_request = tonic::Request::new(InstrumentRequest {});
    Ok(Box::new(
        client.watch_updates(update_request).await?.into_inner(),
    ))
}

pub fn start(uuid: Uuid, url: Url, event_sender: Sender<(Uuid, Event)>) -> Connection {
    let (command_sender, mut command_receiver) = mpsc::channel(100);
    let connection = Connection {
        commands: command_sender,
    };
    async_runtime::spawn(async move {
        'connection: loop {
            event_sender.send((uuid, Event::Connecting)).await;
            let connection = 'connect: loop {
                select! {
                    connection = connect(&url) => {
                        break 'connect connection;
                    }
                    command = command_receiver.recv() => {
                        match command {
                            Some(Command::Disconnect) | None => break 'connection
                        }
                    }
                };
            };
            match connection {
                Ok(mut update_stream) => {
                    event_sender.send((uuid, Event::Connected)).await;
                    // let mut tasks = HashMap::new();
                    loop {
                        select! {
                            update = update_stream.message() => {
                                match update {
                                    Ok(message) => {
                                        if let Some(update) = message {
                                            event_sender.send((uuid, Event::Update(update))).await;
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
                    event_sender.send((uuid, Event::Error(error))).await;
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        }
        event_sender.send((uuid, Event::Disconnected)).await;
    });
    connection
}
