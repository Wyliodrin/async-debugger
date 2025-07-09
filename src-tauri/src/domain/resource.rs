use super::storable::Storable;
use crate::domain::duration::Duration;
use crate::error::Error as TraceError;
use crate::mappers::read_file;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ResourceStatus {
    Ready,
    Pending,
    Disconnected,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub(crate) struct Resource {
    pub app_name: Option<String>,
    pub resource_type: Option<String>,
    pub id: u64,
    pub status: ResourceStatus,
    pub target: Option<String>,
    pub duration: Option<Duration>,
    pub location: Option<String>,
    pub attributes: Option<String>,
}

impl Resource {
    pub fn id(&self) -> String {
        format!("{}.{}", self.app_name.as_ref().unwrap(), self.id)
    }
}

#[async_trait]
impl Storable<HashMap<String, Resource>> for Resource {
    const FILE_EXTENSION: &str = "resources.json";

    async fn load_all(path: String) -> Result<HashMap<String, Resource>, TraceError> {
        let s = read_file(&format!("{}/{}", path, Self::FILE_EXTENSION)).await?;
        let resources =
            serde_json::from_str::<HashMap<String, Resource>>(&s).map_err(TraceError::Serde)?;
        Ok(resources)
    }
}
