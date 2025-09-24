use crate::backend::core::connection_manager::Command;
use crate::backend::core::connection_manager::Connection;
use crate::backend::domain::storable::Storable;
use crate::backend::mappers::read_file;
use crate::utils::common::get_pid_hosting_at;
use crate::utils::common::get_process_start_time;
use crate::utils::error::Error as TraceError;
use async_trait::async_trait;
use log::debug;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use url::Url;
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
    pub(crate) pid: u32,
    id: Uuid,
    pub(crate) title: String,
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

    // This test is needed here in order for the `state_manager::state::tests::test_handle_and_get_pid`
    // test to work
    #[cfg(test)]
    pub fn new_mock(title: String, url: Url, pid: u32) -> Self {
        Application {
            pid,
            id: Uuid::new_v4(),
            title,
            url,
            start_time: "0".parse().unwrap(),
            state: ApplicationState::Enabled,
            connection_status: ConnectionStatus::Disconnected,
            connection: None,
            cpu_usage: 0.0,
            memory_usage: 0,
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
                .map_err(TraceError::Serde)?;

        Ok(apps)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::connection_manager::Command;
    use serde_json::to_string_pretty;
    use std::collections::HashMap;
    use std::fs;
    use std::io::Write;
    use tauri::Url;
    use tempfile::tempdir;
    use tokio::sync::mpsc;
    use uuid::Uuid;

    #[tokio::test]
    async fn new_without_port_fails() {
        let url = Url::parse("http://localhost").unwrap();
        let err = Application::new("NoPort".into(), url).unwrap_err();
        assert!(matches!(err, TraceError::PIDNotFound { .. }));
    }

    #[test]
    fn getters_and_setters_work() {
        let mut app = Application {
            pid: 42,
            id: Uuid::new_v4(),
            title: "MyApp".into(),
            url: Url::parse("http://localhost:6000").unwrap(),
            start_time: "time".into(),
            cpu_usage: 0.0,
            memory_usage: 0,
            state: ApplicationState::Disabled,
            connection_status: ConnectionStatus::Error("ups".into()),
            connection: None,
        };

        app.set_pid(100);
        assert_eq!(app.pid(), 100);

        app.set_cpu_usage(1.23);
        assert!((app._cpu_usage() - 1.23).abs() < f32::EPSILON);

        app.set_memory_usage(2048);
        assert_eq!(app._memory_usage(), 2048);

        app.set_connection_status(ConnectionStatus::Connected);
        assert_eq!(app._connection_status(), &ConnectionStatus::Connected);
    }

    #[tokio::test]
    async fn enable_and_disable_send_disconnect() {
        let mut app = Application {
            pid: 1,
            id: Uuid::new_v4(),
            title: "Test".into(),
            url: Url::parse("http://127.0.0.1:8000").unwrap(),
            start_time: "t0".into(),
            cpu_usage: 0.0,
            memory_usage: 0,
            state: ApplicationState::Disabled,
            connection_status: ConnectionStatus::Disconnected,
            connection: None,
        };

        let (tx, mut rx) = mpsc::channel(1);
        let conn = Connection { commands: tx };

        app.enable(conn.clone());
        assert_eq!(app.state(), ApplicationState::Enabled);
        assert!(app.connection.is_some());

        app.disable().await;
        assert_eq!(rx.recv().await.unwrap(), Command::Disconnect);
        assert_eq!(app.state(), ApplicationState::Disabled);
        assert!(app.connection.is_none());
    }

    #[tokio::test]
    async fn storable_roundtrip_via_temp_file() {
        let uuid = Uuid::new_v4();
        let mut map = HashMap::new();
        let sample = Application {
            pid: 5,
            id: uuid,
            title: "X".into(),
            url: Url::parse("http://127.0.0.1:9000").unwrap(),
            start_time: "t2".into(),
            cpu_usage: 0.0,
            memory_usage: 0,
            state: ApplicationState::Enabled,
            connection_status: ConnectionStatus::Connected,
            connection: None,
        };
        map.insert(uuid, sample.clone());

        let tmpdir = tempdir().unwrap();
        let filepath = tmpdir.path().join(Application::FILE_EXTENSION);
        let json = to_string_pretty(&map).unwrap();
        fs::File::create(&filepath)
            .and_then(|mut f| f.write_all(json.as_bytes()))
            .unwrap();

        let loaded = Application::load_all(tmpdir.path().to_string_lossy().into())
            .await
            .unwrap();
        let got = loaded.get(&uuid).unwrap();
        assert_eq!(got.title(), "X");
        assert_eq!(got.state(), ApplicationState::Enabled);
        assert_eq!(got._connection_status(), &ConnectionStatus::Connected);
    }
}
