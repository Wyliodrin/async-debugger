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
