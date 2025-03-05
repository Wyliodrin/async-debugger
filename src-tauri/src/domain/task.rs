use super::storable::Storable;
use crate::error::Error as TraceError;
use crate::mappers::read_file;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Task {
    pub app_id: Uuid,
    pub id: u64,
    pub tid: Option<u64>,
    pub name: Option<String>,
    pub kind: Option<String>,
}

impl Task {
    pub fn id(&self) -> String {
        format!("{}.{}", self.app_id, self.id)
    }
}

#[async_trait]
impl Storable<Vec<Task>> for Task {
    const FILE_EXTENSION: &str = "tasks.json";

    async fn load_all(path: String) -> Result<Vec<Task>, TraceError> {
        let tasks =
            serde_json::from_str(&read_file(&format!("{}/{}", path, Self::FILE_EXTENSION)).await?)
                .map_err(|err| TraceError::Serde(err))?;

        Ok(tasks)
    }
}
