//! A generic trait for loading domain data from disk.

use crate::error::Error as TraceError;
use async_trait::async_trait;

/// A type that can be loaded from a JSON file named `<TYPE::FILE_EXTENSION>`.
#[async_trait]
pub(crate) trait Storable<T> {
    /// The suffix (including `.json`) for the file holding these items.
    const FILE_EXTENSION: &str;

    /// Load all items of type `T` from the JSON file in `path`.
    ///
    /// # Errors
    ///
    /// Returns a `TraceError` if reading or parsing fails.
    async fn load_all(path: String) -> Result<T, TraceError>;
}
