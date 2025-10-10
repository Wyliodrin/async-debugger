//! Defines the `Poll` domain type and its `Storable` implementation for
//! loading from JSON.

use super::storable::Storable;
use crate::error::Error as TraceError;
use crate::mappers::read_file;
use async_trait::async_trait;
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

/// A single poll (event) captured by the tracing system.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub(crate) struct Poll {
    /// Optional application name that generated this poll.
    pub app_name: Option<String>,
    /// The type or label of the poll.
    pub poll_type: String,
    /// Optional associated resource identifier.
    pub resource_id: Option<u64>,
    /// Optional human‐readable resource name.
    pub resource_name: Option<String>,
    /// Optional associated task identifier.
    pub task_id: Option<u64>,
    /// Optional human‐readable task name.
    pub task_name: Option<String>,
    /// Optional color metadata for a UI.
    pub task_color: Option<String>,
    /// Whether this poll indicates readiness (true) or not.
    pub is_ready: bool,
    /// Optional source code location string.
    pub location: Option<String>,
    /// Optional timestamp when this poll was received.
    pub received_at: Option<DateTime<Local>>,
}

#[async_trait]
impl Storable<Vec<Poll>> for Poll {
    /// JSON file name suffix for polls.
    const FILE_EXTENSION: &str = "polls.json";

    /// Load all polls from `<path>/polls.json`.
    ///
    /// # Errors
    ///
    /// Returns `TraceError::PathNotFound` if the file cannot be read,
    /// or `TraceError::Serde` if JSON parsing fails.
    async fn load_all(path: String) -> Result<Vec<Poll>, TraceError> {
        let s = read_file(&format!("{}/{}", path, Self::FILE_EXTENSION)).await?;
        let polls = serde_json::from_str::<Vec<Poll>>(&s).map_err(TraceError::Serde)?;
        Ok(polls)
    }
}
