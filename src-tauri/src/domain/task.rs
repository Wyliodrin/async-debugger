use super::storable::Storable;
use crate::error::Error as TraceError;
use crate::mappers::read_file;
use async_trait::async_trait;
use console_api::{tasks::Stats, Location};
use serde::{ser::SerializeMap, Deserialize, Serialize, Serializer};
use std::{
    collections::HashMap,
    time::{Duration, SystemTime},
};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Task {
    pub app_id: Uuid,
    pub id: u64,
    pub tid: Option<u64>,
    pub name: Option<String>,
    pub kind: Option<String>,
    pub location: Option<TaskLocation>,

    #[serde(serialize_with = "convert", rename(serialize = "stats_info"))]
    pub task_stats: TaskStats,
}

fn convert<S>(task_stats: &TaskStats, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut map = serializer.serialize_map(Some(4))?;
    let now = SystemTime::now();
    map.serialize_entry("runtime", &total(task_stats, now))?;
    map.serialize_entry("busy", &busy(task_stats, now))?;
    map.serialize_entry("scheduled", &scheduled(task_stats, now))?;
    map.serialize_entry("idle", &idle(task_stats, now))?;
    map.end()
}

/// TODO taken from tokio-console
fn total(task_stats: &TaskStats, since: SystemTime) -> Duration {
    task_stats
        .total
        .or_else(|| since.duration_since(task_stats.created_at).ok())
        .unwrap_or_default()
}

/// TODO taken from tokio-console
fn busy(task_stats: &TaskStats, since: SystemTime) -> Duration {
    if let Some(started) = task_stats.last_poll_started {
        if task_stats.last_poll_started > task_stats.last_poll_ended {
            // in this case the task is being polled at the moment
            let current_time_in_poll = since.duration_since(started).unwrap_or_default();
            return task_stats.busy + current_time_in_poll;
        }
    }
    task_stats.busy
}

/// TODO taken from tokio-console
fn scheduled(task_stats: &TaskStats, since: SystemTime) -> Duration {
    if let Some(wake) = task_stats.last_wake {
        if task_stats.last_wake > task_stats.last_poll_started {
            // In this case the task is scheduled, but has not yet been polled
            let current_time_since_wake = since.duration_since(wake).unwrap_or_default();
            return task_stats.scheduled + current_time_since_wake;
        }
    }
    task_stats.scheduled
}

/// TODO taken from tokio-console
fn idle(task_stats: &TaskStats, since: SystemTime) -> Duration {
    task_stats
        .idle
        .or_else(|| {
            total(task_stats, since)
                .checked_sub(busy(task_stats, since) + scheduled(task_stats, since))
        })
        .unwrap_or_default()
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TaskStats {
    created_at: SystemTime,
    dropped_at: Option<SystemTime>,
    busy: Duration,
    scheduled: Duration,
    last_poll_started: Option<SystemTime>,
    last_poll_ended: Option<SystemTime>,
    idle: Option<Duration>,
    total: Option<Duration>,
    last_wake: Option<SystemTime>,
}

impl From<&Stats> for TaskStats {
    fn from(stats_recv: &Stats) -> Self {
        let created_at = stats_recv
            .created_at
            .expect("task span was never created")
            .try_into()
            .unwrap();

        let dropped_at: Option<SystemTime> = stats_recv.dropped_at.map(|v| v.try_into().unwrap());

        let total = dropped_at.map(|d| d.duration_since(created_at).unwrap_or_default());

        let poll_stats = stats_recv.poll_stats.expect("task should have poll stats");
        let busy = poll_stats.busy_time.map(pb_duration).unwrap_or_default();
        let scheduled = stats_recv
            .scheduled_time
            .map(pb_duration)
            .unwrap_or_default();
        let idle = total.map(|total| total.checked_sub(busy + scheduled).unwrap_or_default());

        Self {
            total,
            idle,
            scheduled,
            busy,
            last_wake: stats_recv.last_wake.map(|v| v.try_into().unwrap()),
            last_poll_started: poll_stats.last_poll_started.map(|v| v.try_into().unwrap()),
            last_poll_ended: poll_stats.last_poll_ended.map(|v| v.try_into().unwrap()),
            created_at,
            dropped_at,
        }
    }
}

fn pb_duration(dur: prost_types::Duration) -> Duration {
    let secs = u64::try_from(dur.seconds).expect("duration should not be negative!");
    let nanos = u64::try_from(dur.nanos).expect("duration should not be negative!");
    Duration::from_secs(secs) + Duration::from_nanos(nanos)
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TaskLocation {
    /// The file path
    pub file: Option<String>,
    /// The Rust module path
    pub module_path: Option<String>,
    /// The line number in the source code file.
    pub line: Option<u32>,
    /// The character in `line`.
    pub column: Option<u32>,
}

impl From<&Location> for TaskLocation {
    fn from(value: &Location) -> Self {
        TaskLocation {
            file: value.file.clone(),
            module_path: value.module_path.clone(),
            line: value.line,
            column: value.column,
        }
    }
}

impl Task {
    pub fn id(&self) -> String {
        format!("{}.{}", self.app_id, self.id)
    }
}

#[async_trait]
impl Storable<HashMap<String, Task>> for Task {
    const FILE_EXTENSION: &str = "tasks.json";

    async fn load_all(path: String) -> Result<HashMap<String, Task>, TraceError> {
        let tasks =
            serde_json::from_str(&read_file(&format!("{}/{}", path, Self::FILE_EXTENSION)).await?)
                .map_err(TraceError::Serde)?;

        Ok(tasks)
    }
}
