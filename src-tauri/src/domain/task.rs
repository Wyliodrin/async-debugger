use super::storable::Storable;
use crate::error::Error as TraceError;
use crate::mappers::read_file;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum TaskState {
  Running,
  Stopped {
    at: DateTime<Utc>,
    reason: Option<String>,
  },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Task {
    pub app_name: Option<String>,
    pub id: u64,
    pub tid: Option<u64>,
    pub name: Option<String>,
    pub kind: Option<String>,
    pub state: TaskState,
}

impl Task {
    pub fn id(&self) -> String {
        format!("{}.{}", self.app_name.as_ref().unwrap(), self.id)
    }
}

#[async_trait]
impl Storable<HashMap<String, Task>> for Task {
    const FILE_EXTENSION: &str = "tasks.json";

    async fn load_all(path: String) -> Result<HashMap<String, Task>, TraceError> {
        let s = read_file(&format!("{}/{}", path, Self::FILE_EXTENSION)).await?;
        let tasks = serde_json::from_str::<HashMap<String, Task>>(&s)
            .map_err(TraceError::Serde)?;
        Ok(tasks)
    }
}
