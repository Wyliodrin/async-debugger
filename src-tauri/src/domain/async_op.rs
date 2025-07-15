use super::storable::Storable;
use crate::error::Error as TraceError;
use crate::mappers::read_file;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) struct AsyncOp {
    pub id: u64,
    pub resource_id: u64,
    pub resource_target: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) struct TimeStamp {
    pub seconds: i64,
    pub nanos: i32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) struct CPUOverview {
    pub started_at: Option<TimeStamp>,
    pub stopped_at: Option<TimeStamp>,
    pub resource_target: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) struct TaskOp {
    pub task_id: u64,
    pub task_name: Option<String>,
    pub task_color: Option<String>,
    pub operations: Vec<CPUOverview>,
}

#[async_trait]
impl Storable<HashMap<String, AsyncOp>> for AsyncOp {
    const FILE_EXTENSION: &str = "async_ops.json";

    async fn load_all(path: String) -> Result<HashMap<String, AsyncOp>, TraceError> {
        let s = read_file(&format!("{}/{}", path, Self::FILE_EXTENSION)).await?;
        let async_ops =
            serde_json::from_str::<HashMap<String, AsyncOp>>(&s).map_err(TraceError::Serde)?;
        Ok(async_ops)
    }
}

#[async_trait]
impl Storable<HashMap<String, TaskOp>> for TaskOp {
    const FILE_EXTENSION: &str = "tasks_ops.json";

    async fn load_all(path: String) -> Result<HashMap<String, TaskOp>, TraceError> {
        let s = read_file(&format!("{}/{}", path, Self::FILE_EXTENSION)).await?;
        let tasks_op =
            serde_json::from_str::<HashMap<String, TaskOp>>(&s).map_err(TraceError::Serde)?;
        Ok(tasks_op)
    }
}
