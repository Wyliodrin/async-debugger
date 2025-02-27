use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("URL: {0}")]
    Url(#[from] url::ParseError),
    #[error("Application with id {0} is already connected")]
    ApplicationAlreadyConnected(Uuid),
    #[error("TODO: add message for me")]
    Anyhow(#[from] anyhow::Error),
    #[error("Path {0} not found")]
    PathNotFound(String),
    #[error("Serde error encountered: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("Cannot create the storage directory at path {path} due to {error}")]
    CannotCreateStorage { error: anyhow::Error, path: String },
}

impl serde::Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_str(self.to_string().as_ref())
    }
}
