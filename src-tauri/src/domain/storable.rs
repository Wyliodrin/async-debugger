use crate::error::Error as TraceError;
use async_trait::async_trait;

#[async_trait]
pub(crate) trait Storable<T> {
    const FILE_EXTENSION: &str;

    async fn load_all(path: String) -> Result<T, TraceError>;
}
