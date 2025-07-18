//! Defines resources tracked in the trace domain and implements storage.

use super::storable::Storable;
use crate::domain::duration::Duration;
use crate::error::Error as TraceError;
use crate::mappers::read_file;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Possible lifecycle states of a traced resource.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ResourceStatus {
    /// Resource is fully initialized and ready.
    Ready,
    /// Resource is pending initialization.
    Pending,
    /// Resource has been dropped or cleaned up.
    Dropped,
}

/// A tracked resource in the application trace.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub(crate) struct Resource {
    /// Optional application name this resource belongs to.
    pub app_name: Option<String>,
    /// Optional kind/type string of the resource.
    pub resource_type: Option<String>,
    /// Unique numeric ID of the resource.
    pub id: u64,
    /// Current state of the resource.
    pub status: ResourceStatus,
    /// Optional target or descriptor.
    pub target: Option<String>,
    /// Optional lifetime duration of the resource.
    pub duration: Option<Duration>,
    /// Optional source‐code location.
    pub location: Option<String>,
    /// Optional extra attributes in JSON or plain text.
    pub attributes: Option<String>,
}

impl Resource {
    /// Returns a string identifier combining `app_name` and `id`.
    ///
    /// # Panics
    ///
    /// Panics if `app_name` is `None`.
    pub fn id(&self) -> String {
        format!("{}.{}", self.app_name.as_ref().unwrap(), self.id)
    }
}

#[async_trait]
impl Storable<HashMap<String, Resource>> for Resource {
    /// JSON file suffix used when persisting resources.
    const FILE_EXTENSION: &str = "resources.json";

    /// Load all resources from `<path>/resources.json`.
    ///
    /// # Errors
    ///
    /// - `TraceError::PathNotFound` if the file cannot be read.
    /// - `TraceError::Serde` if JSON fails to deserialize.
    async fn load_all(path: String) -> Result<HashMap<String, Resource>, TraceError> {
        let s = read_file(&format!("{}/{}", path, Self::FILE_EXTENSION)).await?;
        let resources =
            serde_json::from_str::<HashMap<String, Resource>>(&s).map_err(TraceError::Serde)?;
        Ok(resources)
    }
}
