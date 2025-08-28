//! Defines the `Task` domain type (with run/schedule/idleness stats) and
//! implements `Storable` to read tasks from JSON.

use super::storable::Storable;
use crate::domain::{duration::Duration, has_app_name::HasAppName};
use crate::error::Error as TraceError;
use crate::mappers::read_file;
use async_trait::async_trait;
use chrono::{DateTime, Duration as ChronoDuration, Utc};
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
    /// Optional scheduled timestamp
    pub scheduled_at: Option<DateTime<Utc>>,
    /// Optional woken timestamp
    pub woken_at: Option<DateTime<Utc>>,
    /// Optional starved boolean
    pub starved: Option<bool>,
    /// Optional starved duration
    pub starved_since_ms: Option<i64>,
}

fn duration_to_chrono(d: &Duration) -> ChronoDuration {
    ChronoDuration::seconds(d.seconds) + ChronoDuration::nanoseconds(d.nanos as i64)
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

    /// Compute starvation fields for UI.
    /// thresh_ms = threshold in milliseconds to consider a task starved.
    pub fn compute_starvation(&mut self, thresh_ms: i64) -> bool {
        fn set_fields(task: &mut Task, starved: Option<bool>, since_ms: Option<i64>) -> bool {
            let changed = task.starved != starved || task.starved_since_ms != since_ms;
            task.starved = starved;
            task.starved_since_ms = since_ms;
            changed
        }

        let now = Utc::now();

        let candidate: (bool, Option<i64>) = if let Some(w) = self.woken_at {
            let mut age = now.signed_duration_since(w);
            if age < ChronoDuration::zero() {
                age = ChronoDuration::zero();
            }
            let age_ms = age.num_milliseconds();
            (age_ms > thresh_ms, Some(age_ms))
        } else if let Some(s) = self.scheduled_at {
            let mut age = now.signed_duration_since(s);
            if age < ChronoDuration::zero() {
                age = ChronoDuration::zero();
            }
            let age_ms = age.num_milliseconds();
            (age_ms > thresh_ms, Some(age_ms))
        } else if let (Some(runtime), Some(busy)) = (self.runtime.clone(), self.busy.clone()) {
            let runtime_ch = duration_to_chrono(&runtime);
            let busy_ch = duration_to_chrono(&busy);
            let mut inferred_idle = runtime_ch - busy_ch;
            if inferred_idle < ChronoDuration::zero() {
                inferred_idle = ChronoDuration::zero();
            }
            let idle_ms = inferred_idle.num_milliseconds();
            (idle_ms > thresh_ms, Some(idle_ms))
        } else {
            (false, None)
        };

        set_fields(self, Some(candidate.0), candidate.1)
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_woken_at_prefers_woken() {
        let mut t = Task {
            app_name: Some("app4".into()),
            id: 99,
            tid: Some(4096),
            name: Some("cron-runner".into()),
            color: Some("#34D399".into()),
            kind: Some("scheduled".into()),
            state: TaskState::Stopped {
                at: "2025-07-01T08:59:30Z".parse::<DateTime<Utc>>().unwrap(),
                reason: Some("completed scheduled run".into()),
            },
            runtime: Some(Duration::new(3600, 0)),
            scheduled: Some(Duration::new(3660, 0)),
            idle: Some(Duration::new(60, 0)),
            busy: Some(Duration::new(3600, 0)),
            location: Some("src/scheduler/mod.rs:512".into()),
            created_at: Some("2025-07-01T08:00:00Z".into()),
            scheduled_at: "2025-07-01T09:01:00Z".parse::<DateTime<Utc>>().ok(),
            woken_at: "2025-07-01T08:59:30Z".parse::<DateTime<Utc>>().ok(),
            starved: Some(false),
            starved_since_ms: None,
        };
        t.woken_at = Some(Utc::now() - chrono::Duration::milliseconds(500));
        t.scheduled_at = Some(Utc::now() - chrono::Duration::milliseconds(1000));
        t.compute_starvation(200);
        assert_eq!(t.starved, Some(true));
        assert!(t.starved_since_ms.unwrap() >= 500);
    }

    #[test]
    fn test_scheduled_only() {
        let mut t = Task {
            app_name: Some("app1".into()),
            id: 42,
            tid: Some(2048),
            name: Some("worker-42".into()),
            color: Some("#7A9CF7".into()),
            kind: Some("io-bound".into()),
            state: TaskState::Running,
            runtime: Some(Duration::new(12, 500_000_000)),
            scheduled: Some(Duration::new(20, 0)),
            idle: Some(Duration::new(7, 500_000_000)),
            busy: Some(Duration::new(12, 500_000_000)),
            location: Some("src/worker/mod.rs:128".into()),
            created_at: Some("2025-08-27T12:34:56Z".into()),
            scheduled_at: "2025-08-27T12:35:56Z".parse::<DateTime<Utc>>().ok(),
            woken_at: "2025-08-27T12:34:46Z".parse::<DateTime<Utc>>().ok(),
            starved: Some(false),
            starved_since_ms: None,
        };
        t.woken_at = None;
        t.scheduled_at = Some(Utc::now() - chrono::Duration::milliseconds(100));
        t.compute_starvation(200);
        assert_eq!(t.starved, Some(false));
    }

    #[test]
    fn test_fallback_runtime_busy() {
        let mut t = Task {
            app_name: Some("app2".into()),
            id: 43,
            tid: Some(2050),
            name: Some("worker-43".into()),
            color: Some("#F7A97A".into()),
            kind: Some("cpu-bound".into()),
            state: TaskState::Running,
            runtime: Some(Duration::new(3, 250_000_000)),
            scheduled: Some(Duration::new(10, 0)),
            idle: Some(Duration::new(6, 750_000_000)),
            busy: Some(Duration::new(3, 250_000_000)),
            location: Some("src/worker/compute.rs:256".into()),
            created_at: Some("2025-08-27T11:00:00Z".into()),
            scheduled_at: None,
            woken_at: None,
            starved: Some(true),
            starved_since_ms: Some(120_000),
        };

        t.runtime = Some(Duration::new(5, 0));
        t.busy = Some(Duration::new(1, 0));

        t.compute_starvation(3000);

        assert_eq!(t.starved, Some(true));
        assert!(t.starved_since_ms.unwrap() >= 4000);
    }

    #[test]
    fn test_no_data() {
        let mut t = Task {
            app_name: None,
            id: 44,
            tid: None,
            name: Some("helper".into()),
            color: None,
            kind: Some("network".into()),
            state: TaskState::Running,
            runtime: None,
            scheduled: Some(Duration::new(1, 0)),
            idle: Some(Duration::new(0, 550_000_000)),
            busy: None,
            location: Some("src/net/mod.rs:48".into()),
            created_at: Some("2025-08-27T00:00:00Z".into()),
            scheduled_at: None,
            woken_at: None,
            starved: Some(false),
            starved_since_ms: None,
        };

        t.compute_starvation(200);

        assert_eq!(t.starved, Some(false));
        assert_eq!(t.starved_since_ms, None);
    }
}
