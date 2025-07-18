use super::storable::Storable;
use crate::common::{get_pid_hosting_at, get_process_start_time};
use crate::error::Error as TraceError;
use crate::mappers::read_file;
use crate::state_manager::connection_manager::{Command, Connection};
use async_trait::async_trait;
use log::debug;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::Url;
use uuid::Uuid;

/// Whether an application is currently enabled (connected)
/// or disabled (no active connection).
#[derive(Default, Debug, Serialize, Deserialize, PartialEq, Copy, Clone)]
pub(crate) enum ApplicationState {
    #[default]
    Disabled,
    Enabled,
}

/// Status of the underlying connection channel.
#[derive(Default, Debug, Serialize, Deserialize, PartialEq, Clone)]
pub(crate) enum ConnectionStatus {
    /// No channel created yet.
    #[default]
    Disconnected,
    /// In the process of establishing.
    Connecting,
    /// Successfully connected.
    Connected,
    /// Connection errored; contains the error message.
    Error(String),
}

/// Represents a tracked application: its metadata,
/// current process info, and connection state.
///
/// Serialized to disk for state persistence.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub(crate) struct Application {
    pid: u32,
    id: Uuid,
    title: String,
    url: Url,
    start_time: String,
    cpu_usage: f32,
    memory_usage: u64,
    state: ApplicationState,
    connection_status: ConnectionStatus,
    #[serde(skip)]
    connection: Option<Connection>,
}

impl Application {
    /// Create a new application record by discovering the PID
    /// and its start time from the given URL.
    ///
    /// # Errors
    ///
    /// Returns [`TraceError::PIDNotFound`] if no process
    /// is listening on that URL, or if the start time cannot
    /// be retrieved.
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
            connection_status: ConnectionStatus::Disconnected,
            connection: None,
            cpu_usage: 0.0,
            memory_usage: 0,
        })
    }

    /// Current operating system PID of the running process.
    pub fn pid(&self) -> u32 {
        self.pid
    }

    /// Update the stored PID (for reattach scenarios).
    pub fn set_pid(&mut self, pid: u32) {
        debug!("Setting the pid to {}", pid);
        self.pid = pid;
    }

    /// Globally unique identifier for this application.
    pub fn id(&self) -> &Uuid {
        &self.id
    }

    /// Title as provided by the user.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// The original URL endpoint used to connect.
    pub fn url(&self) -> Url {
        self.url.clone()
    }

    /// Whether this application is enabled or disabled.
    pub fn state(&self) -> ApplicationState {
        self.state
    }

    pub fn _cpu_usage(&self) -> f32 {
        self.cpu_usage
    }

    /// Update the CPU‐usage statistic.
    pub fn set_cpu_usage(&mut self, usage: f32) {
        self.cpu_usage = usage;
    }

    pub fn _memory_usage(&self) -> u64 {
        self.memory_usage
    }

    /// Update the memory‐usage statistic.
    pub fn set_memory_usage(&mut self, usage: u64) {
        self.memory_usage = usage;
    }

    pub fn _connection_status(&self) -> &ConnectionStatus {
        &self.connection_status
    }

    /// Update the live connection status.
    pub fn set_connection_status(&mut self, conn_status: ConnectionStatus) {
        self.connection_status = conn_status;
    }

    /// Mark as enabled and stash the live connection object.
    pub fn enable(&mut self, connection: Connection) {
        self.state = ApplicationState::Enabled;
        self.connection = Some(connection);
        debug!("Stored connection");
    }

    /// Mark as disabled and send a `Disconnect` command.
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
    /// Load all applications from disk under `path`.
    async fn load_all(path: String) -> Result<HashMap<Uuid, Application>, TraceError> {
        let apps =
            serde_json::from_str(&read_file(&format!("{}/{}", path, Self::FILE_EXTENSION)).await?)
                .map_err(|err| TraceError::Serde(err))?;

        Ok(apps)
    }
}
