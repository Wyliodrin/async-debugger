use std::collections::HashMap;

use super::storable::Storable;
use crate::error::Error as TraceError;
use crate::mappers::read_file;
use crate::state_manager::connection_manager::{Command, Connection};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tauri::Url;
use uuid::Uuid;

#[derive(Default, Debug, Serialize, Deserialize, PartialEq, Copy, Clone)]
pub(crate) enum ApplicationState {
    #[default]
    Disabled,
    Enabled,
}

/// Application tracked by the application
///
/// Keeps app's metadatas and current state
#[derive(Clone, Serialize, Deserialize, Debug)]
pub(crate) struct Application {
    pid: u32,
    id: Uuid,
    title: String,
    url: Url,
    state: ApplicationState,

    #[serde(skip)]
    connection: Option<Connection>,
}

impl Application {
    pub fn new(pid: u32, title: String, url: Url) -> Application {
        Application {
            pid,
            id: Uuid::new_v4(),
            title,
            url,
            state: ApplicationState::Disabled,

            connection: None,
        }
    }

    pub fn pid(&self) -> u32 {
        self.pid
    }

    pub fn id(&self) -> &Uuid {
        &self.id
    }

    pub fn _title(&self) -> &str {
        &self.title
    }

    pub fn url(&self) -> &Url {
        &self.url
    }

    pub fn state(&self) -> ApplicationState {
        self.state
    }

    // vreau sa vad info pentru aplicatia asta
    pub fn enable(&mut self, connection: Connection) {
        if self.state == ApplicationState::Disabled {
            self.state = ApplicationState::Enabled;
            self.connection = Some(connection);
        }
    }

    pub async fn disable(&mut self) {
        if let Some(connection) = self.connection.take() {
            connection.commands.send(Command::Disconnect).await.ok();
            self.state = ApplicationState::Disabled;
        }
    }
}

#[async_trait]
impl Storable<HashMap<Uuid, Application>> for Application {
    const FILE_EXTENSION: &str = "applications.json";
    async fn load_all(path: String) -> Result<HashMap<Uuid, Application>, TraceError> {
        let apps =
            serde_json::from_str(&read_file(&format!("{}/{}", path, Self::FILE_EXTENSION)).await?)
                .map_err(|err| TraceError::Serde(err))?;

        Ok(apps)
    }
}
