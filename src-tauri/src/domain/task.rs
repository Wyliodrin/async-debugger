//! Defines the `Task` domain type (with run/schedule/idleness stats) and
//! implements `Storable` to read tasks from JSON.

use super::storable::Storable;
use crate::domain::{duration::Duration, has_app_name::HasAppName};
use crate::error::Error as TraceError;
use crate::mappers::read_file;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Lifecycle state of a task.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum TaskState {
    /// Task is running.
    Running,
    /// Task has stopped at a given timestamp for an optional reason.
    Stopped {
        /// When the stop occurred.
        at: DateTime<Utc>,
        /// Optional human‐readable explanation.
        reason: Option<String>,
    },
}

/// A traced task with timing and scheduling information.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Task {
    /// Optional application name the task belongs to.
    pub app_name: Option<String>,
    /// Unique numeric ID of the task.
    pub id: u64,
    /// Optional OS thread identifier (tid).
    pub tid: Option<u64>,
    /// Optional human‐readable task name.
    pub name: Option<String>,
    /// Optional color hint for UI.
    pub color: Option<String>,
    /// Optional category/kind string.
    pub kind: Option<String>,
    /// Current state (running or stopped).
    pub state: TaskState,
    /// Total runtime duration (if known).
    pub runtime: Option<Duration>,
    /// Total scheduled time.
    pub scheduled: Option<Duration>,
    /// Total idle time.
    pub idle: Option<Duration>,
    /// Total busy time.
    pub busy: Option<Duration>,
    /// Optional source location.
    pub location: Option<String>,
    /// Optional creation timestamp.
    pub created_at: Option<String>,
}

impl Task {
    /// Returns a composite string identifier `<app_name>.<id>`.
    ///
    /// # Panics
    ///
    /// Panics if `app_name` is `None`.
    pub fn id(&self) -> String {
        format!("{}.{}", self.app_name.as_ref().unwrap(), self.id)
    }
}

#[async_trait]
impl Storable<HashMap<String, Task>> for Task {
    /// The file suffix under which tasks are stored.
    const FILE_EXTENSION: &str = "tasks.json";

    /// Load all tasks from `<path>/tasks.json`.
    ///
    /// # Errors
    ///
    /// - `TraceError::PathNotFound` if the file cannot be read.
    /// - `TraceError::Serde` if the JSON is malformed.
    async fn load_all(path: String) -> Result<HashMap<String, Task>, TraceError> {
        let s = read_file(&format!("{}/{}", path, Self::FILE_EXTENSION)).await?;
        let tasks = serde_json::from_str::<HashMap<String, Task>>(&s).map_err(TraceError::Serde)?;
        Ok(tasks)
    }
}

impl HasAppName for Task {
    fn set_app_name(&mut self, new_app_name: String) {
        self.app_name = Some(new_app_name.clone());
    }
}
