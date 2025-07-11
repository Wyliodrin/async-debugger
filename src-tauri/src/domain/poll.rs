use super::storable::Storable;
use crate::error::Error as TraceError;
use crate::mappers::read_file;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub(crate) struct Poll {
    pub app_name: Option<String>,
    pub poll_type: String,
    pub resource_id: Option<u64>,
    pub task_id: Option<u64>,
    pub is_ready: bool,
    pub location: Option<String>,
    pub received_at: Option<String>,
}

#[async_trait]
impl Storable<Vec<Poll>> for Poll {
    const FILE_EXTENSION: &str = "polls.json";

    async fn load_all(path: String) -> Result<Vec<Poll>, TraceError> {
        let s = read_file(&format!("{}/{}", path, Self::FILE_EXTENSION)).await?;
        let polls = serde_json::from_str::<Vec<Poll>>(&s).map_err(TraceError::Serde)?;
        Ok(polls)
    }
}
