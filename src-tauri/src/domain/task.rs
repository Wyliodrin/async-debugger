//! Defines the `Task` domain type (with run/schedule/idleness stats) and
//! implements `Storable` to read tasks from JSON.

use super::storable::Storable;
use crate::domain::{duration::Duration, has_app_name::HasAppName};
use crate::error::Error as TraceError;
use crate::mappers::read_file;
use crate::warnings::TaskWarnings;
use async_trait::async_trait;
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::SystemTime;

/// Lifecycle state of a task.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum TaskState {
    /// Task is running.
    Running,
    /// Task has stopped at a given timestamp for an optional reason.
    Stopped {
        /// When the stop occurred.
        at: DateTime<Local>,
        /// Optional human‐readable explanation.
        reason: Option<String>,
    },
    Starved,
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
    /// The size of the future driving the task
    pub size_bytes: Option<usize>,
    /// The original size of the future (before runtime auto-boxing)
    pub original_size_bytes: Option<usize>,

    pub stats: TaskStats,

    pub warnings: TaskWarnings,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TaskStats {
    pub wakes: u64,

    pub self_wakes: u64,

    pub waker_clones: u64,

    pub waker_drops: u64,

    pub polls: u64,

    pub last_wake: Option<SystemTime>,

    pub last_poll_started: Option<SystemTime>,

    pub last_poll_ended: Option<SystemTime>,
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

    pub(crate) fn wakes(&self) -> u64 {
        self.stats.wakes
    }

    pub(crate) fn self_wakes(&self) -> u64 {
        self.stats.self_wakes
    }
    pub(crate) fn waker_clones(&self) -> u64 {
        self.stats.waker_clones
    }

    pub(crate) fn waker_drops(&self) -> u64 {
        self.stats.waker_drops
    }

    pub(crate) fn total_polls(&self) -> u64 {
        self.stats.polls
    }

    pub(crate) fn size_bytes(&self) -> Option<usize> {
        self.size_bytes
    }

    pub(crate) fn original_size_bytes(&self) -> Option<usize> {
        self.original_size_bytes
    }

    pub(crate) fn check_warnings(&self) -> Vec<String> {
        self.warnings.check(&self)
    }

    pub(crate) fn is_blocking(&self) -> bool {
        matches!(self.kind.as_deref(), Some("block_on") | Some("blocking"))
    }

    pub(crate) fn is_completed(&self) -> bool {
        matches!(self.state, TaskState::Stopped { at: _, reason: _ })
    }

    pub(crate) fn waker_count(&self) -> u64 {
        self.waker_clones().saturating_sub(self.waker_drops())
    }

    pub(crate) fn self_wake_percent(&self) -> u64 {
        let total = self.wakes();
        if total == 0 {
            0
        } else {
            ((self.self_wakes() as f64 / total as f64) * 100.0) as u64
        }
    }

    pub(crate) fn busy(&self, since: SystemTime) -> std::time::Duration {
        if let Some(busy_duration) = self.busy.clone() {
            let busy_time =
                std::time::Duration::new(busy_duration.seconds as u64, busy_duration.nanos as u32);
            if let Some(started) = self.stats.last_poll_started {
                if self.stats.last_poll_started > self.stats.last_poll_ended {
                    // in this case the task is being polled at the moment
                    let current_time_in_poll = since.duration_since(started).unwrap_or_default();
                    return busy_time + current_time_in_poll;
                }
            }
            busy_time
        } else {
            std::time::Duration::new(0, 0)
        }
    }

    pub(crate) fn is_running(&self) -> bool {
        self.stats.last_poll_started > self.stats.last_poll_ended
    }

    pub(crate) fn is_awakened(&self) -> bool {
        // Before the first poll, the task is waiting on the executor to run it
        // for the first time.
        //self.total_polls() == 0 || self.stats.last_wake() > self.stats.last_poll_started
        true
    }

    pub(crate) fn is_starved(&self) -> bool {
        self.stats.last_wake > self.stats.last_poll_started
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
