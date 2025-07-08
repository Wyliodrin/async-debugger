use url::Url;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("URL: {0}")]
    Url(#[from] url::ParseError),
    #[error("ApplicationAlreadyConnected: Application with id {0} is already connected")]
    ApplicationAlreadyConnected(String),
    #[error("TODO: add message for me")]
    Anyhow(#[from] anyhow::Error),
    #[error("Path {0} not found")]
    PathNotFound(String),
    #[error("Serde error encountered: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("Cannot create the storage directory at path {path} due to {error}")]
    CannotCreateStorage { error: anyhow::Error, path: String },
    #[error("PIDNotFound: Could not find the PID of an application hosting at {url}")]
    PIDNotFound { url: Url },
    #[error("Could not find the application with the PID {pid}")]
    CannotReadProcessInfo { pid: u32 },
    #[error(
        "CannotCreateChannelForApp: Cannot create a channel for the application with url {url}"
    )]
    CannotCreateChannelForApp { url: String },
}

impl serde::Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_str(self.to_string().as_ref())
    }
}
