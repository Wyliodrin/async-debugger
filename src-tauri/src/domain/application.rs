use super::storable::Storable;
use crate::common::{get_pid_hosting_at, get_process_start_time};
use crate::error::Error as TraceError;
use crate::mappers::read_file;
use crate::state_manager::connection_manager::{Command, Connection};
use async_trait::async_trait;
use chrono::{DateTime, Local};
use log::debug;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
    start_time: DateTime<Local>,
    state: ApplicationState,
    cpu_usage: f32,
    memory_usage: u64,

    #[serde(skip)]
    connection: Option<Connection>,
}

impl Application {
    pub fn new(title: String, url: Url) -> Result<Application, TraceError> {
        // Find the PID of the app
        let pid =
            get_pid_hosting_at(url.clone()).ok_or(TraceError::PIDNotFound { url: url.clone() })?;
        let start_time =
            get_process_start_time(pid).ok_or(TraceError::PIDNotFound { url: url.clone() })?;

        Ok(Application {
            pid,
            id: Uuid::new_v4(),
            title,
            url,
            start_time,
            state: ApplicationState::Enabled,
            connection: None,
            cpu_usage: 0.0,
            memory_usage: 0,
        })
    }

    pub fn pid(&self) -> u32 {
        self.pid
    }

    pub fn set_pid(&mut self, pid: u32) {
        debug!("Setting the pid to {}", pid);
        self.pid = pid;
    }

    pub fn id(&self) -> &Uuid {
        &self.id
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn url(&self) -> Url {
        self.url.clone()
    }

    pub fn state(&self) -> ApplicationState {
        self.state
    }

    pub fn cpu_usage(&self) -> f32 {
        self.cpu_usage
    }

    pub fn set_cpu_usage(&mut self, usage: f32) {
        self.cpu_usage = usage;
    }

    pub fn memory_usage(&self) -> u64 {
        self.memory_usage
    }

    pub fn set_memory_usage(&mut self, usage: u64) {
        self.memory_usage = usage;
    }

    // vreau sa vad info pentru aplicatia asta
    pub fn enable(&mut self, connection: Connection) {
        self.state = ApplicationState::Enabled;
        self.connection = Some(connection);
        debug!("Stored connection");
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
