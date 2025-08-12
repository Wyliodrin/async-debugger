//! This module defines asynchronous‐operation–related domain types (`AsyncOp`, `TimeStamp`, `CPUOverview`, `TaskOp`)
//! and implements the `Storable` trait so that they can be loaded from JSON files.

use super::storable::Storable;
use crate::error::Error as TraceError;
use crate::mappers::read_file;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a single asynchronous operation in the trace domain.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) struct AsyncOp {
    /// Unique identifier of the asynchronous operation.
    pub id: u64,

    /// Identifier of the resource this operation targets.
    pub resource_id: u64,

    /// Optional name or target descriptor of the resource.
    pub resource_target: Option<String>,
}

/// A timestamp with seconds and nanoseconds components.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) struct TimeStamp {
    /// Whole seconds since UNIX epoch.
    pub seconds: i64,

    /// Nanosecond part of the timestamp (0–999_999_999).
    pub nanos: i32,
}

/// Overview of CPU usage for a given operation slice.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) struct CPUOverview {
    /// When the CPU slice started (if known).
    pub started_at: Option<TimeStamp>,

    /// When the CPU slice stopped (if known).
    pub stopped_at: Option<TimeStamp>,

    /// Optional resource target associated with this CPU slice.
    pub resource_target: Option<String>,
}

/// Aggregated CPU overview operations grouped by task.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) struct TaskOp {
    /// Identifier of the task.
    pub task_id: u64,

    /// Optional name of the task.
    pub task_name: Option<String>,

    /// Optional color designation for UI displays.
    pub task_color: Option<String>,

    /// List of CPU‐usage slices.
    pub operations: Vec<CPUOverview>,
}

#[async_trait]
impl Storable<HashMap<String, AsyncOp>> for AsyncOp {
    /// File extension (in the storage folder) where async operations are serialized.
    const FILE_EXTENSION: &str = "async_ops.json";

    /// Load all `AsyncOp` records from `<path>/async_ops.json`.
    ///
    /// # Errors
    ///
    /// If the file cannot be read, returns `TraceError::PathNotFound`.
    /// If JSON deserialization fails, returns `TraceError::Serde`.
    async fn load_all(path: String) -> Result<HashMap<String, AsyncOp>, TraceError> {
        let s = read_file(&format!("{}/{}", path, Self::FILE_EXTENSION)).await?;
        let async_ops =
            serde_json::from_str::<HashMap<String, AsyncOp>>(&s).map_err(TraceError::Serde)?;
        Ok(async_ops)
    }
}

#[async_trait]
impl Storable<HashMap<String, TaskOp>> for TaskOp {
    /// File extension (in the storage folder) where task operations are serialized.
    const FILE_EXTENSION: &str = "tasks_ops.json";

    /// Load all `TaskOp` records from `<path>/tasks_ops.json`.
    ///
    /// # Errors
    ///
    /// If the file cannot be read, returns `TraceError::PathNotFound`.
    /// If JSON deserialization fails, returns `TraceError::Serde`.
    async fn load_all(path: String) -> Result<HashMap<String, TaskOp>, TraceError> {
        let s = read_file(&format!("{}/{}", path, Self::FILE_EXTENSION)).await?;
        let tasks_op =
            serde_json::from_str::<HashMap<String, TaskOp>>(&s).map_err(TraceError::Serde)?;
        Ok(tasks_op)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Error as TraceError;
    use serde_json::to_string_pretty;
    use std::collections::HashMap;
    use std::fs::{self, File};
    use std::io::Write;
    use tempfile::tempdir;
    use tokio;

    #[tokio::test]
    async fn asyncop_load_all_success() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join(AsyncOp::FILE_EXTENSION);

        // sample data
        let mut map: HashMap<String, AsyncOp> = HashMap::new();
        map.insert(
            "first".to_string(),
            AsyncOp {
                id: 1,
                resource_id: 10,
                resource_target: Some("targetA".into()),
            },
        );
        map.insert(
            "second".to_string(),
            AsyncOp {
                id: 2,
                resource_id: 20,
                resource_target: None,
            },
        );

        let json = to_string_pretty(&map).unwrap();
        let mut file = File::create(&file_path).unwrap();
        file.write_all(json.as_bytes()).unwrap();

        let loaded = AsyncOp::load_all(dir.path().to_string_lossy().into())
            .await
            .unwrap();

        assert_eq!(loaded.len(), 2);
        let first = loaded.get("first").unwrap();
        assert_eq!(first.id, 1);
        assert_eq!(first.resource_id, 10);
        assert_eq!(first.resource_target.as_deref(), Some("targetA"));

        let second = loaded.get("second").unwrap();
        assert_eq!(second.id, 2);
        assert_eq!(second.resource_id, 20);
        assert!(second.resource_target.is_none());
    }

    #[tokio::test]
    async fn asyncop_load_all_file_not_found() {
        // a directory that doesn't contain our jsons
        let dir = tempdir().unwrap();
        let err = AsyncOp::load_all(dir.path().to_string_lossy().into())
            .await
            .unwrap_err();
        assert!(matches!(err, TraceError::PathNotFound(_)));
    }

    #[tokio::test]
    async fn asyncop_load_all_bad_json() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join(AsyncOp::FILE_EXTENSION);
        fs::write(&file_path, "not a valid json").unwrap();

        let err = AsyncOp::load_all(dir.path().to_string_lossy().into())
            .await
            .unwrap_err();
        assert!(matches!(err, TraceError::Serde(_)));
    }

    #[tokio::test]
    async fn taskop_load_all_success() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join(TaskOp::FILE_EXTENSION);

        let ts1 = TimeStamp {
            seconds: 100,
            nanos: 500,
        };
        let ts2 = TimeStamp {
            seconds: 200,
            nanos: 0,
        };
        let cpu1 = CPUOverview {
            started_at: Some(ts1.clone()),
            stopped_at: Some(ts2.clone()),
            resource_target: Some("res1".into()),
        };
        let cpu2 = CPUOverview {
            started_at: None,
            stopped_at: None,
            resource_target: None,
        };

        let mut map: HashMap<String, TaskOp> = HashMap::new();
        map.insert(
            "taskA".into(),
            TaskOp {
                task_id: 42,
                task_name: Some("Alpha".into()),
                task_color: Some("#fff".into()),
                operations: vec![cpu1.clone(), cpu2.clone()],
            },
        );

        let json = to_string_pretty(&map).unwrap();
        let mut file = File::create(&file_path).unwrap();
        file.write_all(json.as_bytes()).unwrap();

        let loaded = TaskOp::load_all(dir.path().to_string_lossy().into())
            .await
            .unwrap();

        assert_eq!(loaded.len(), 1);
        let got = loaded.get("taskA").unwrap();
        assert_eq!(got.task_id, 42);
        assert_eq!(got.task_name.as_deref(), Some("Alpha"));
        assert_eq!(got.task_color.as_deref(), Some("#fff"));
        assert_eq!(got.operations.len(), 2);
        assert_eq!(got.operations[0].started_at.as_ref().unwrap().seconds, 100);
        assert_eq!(got.operations[1].resource_target, None);
    }

    #[tokio::test]
    async fn taskop_load_all_file_not_found() {
        let dir = tempdir().unwrap();
        let err = TaskOp::load_all(dir.path().to_string_lossy().into())
            .await
            .unwrap_err();
        assert!(matches!(err, TraceError::PathNotFound(_)));
    }

    #[tokio::test]
    async fn taskop_load_all_bad_json() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join(TaskOp::FILE_EXTENSION);
        fs::write(&file_path, "{not: valid, json]").unwrap();

        let err = TaskOp::load_all(dir.path().to_string_lossy().into())
            .await
            .unwrap_err();
        assert!(matches!(err, TraceError::Serde(_)));
    }
}
