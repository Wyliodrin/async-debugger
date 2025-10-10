use url::Url;

/// Represents all errors that can occur in the application.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// URL parsing failed.
    #[error("URL: {0}")]
    Url(#[from] url::ParseError),

    /// An application with the given ID is already connected.
    #[error("ApplicationAlreadyConnected: Application with id {0} is already connected")]
    ApplicationAlreadyConnected(String),

    /// A catch-all error for uses of `anyhow::Error`.
    #[error("The app encountered a problem")]
    Anyhow(#[from] anyhow::Error),

    /// The specified file or directory path was not found.
    #[error("Path {0} not found")]
    PathNotFound(String),

    /// A JSON serialization or deserialization error.
    #[error("Serde error encountered: {0}")]
    Serde(#[from] serde_json::Error),

    /// Failed to create the storage directory at the given path.
    #[error("Cannot create the storage directory at path {path} due to {error}")]
    CannotCreateStorage {
        /// The underlying error that prevented creation.
        error: anyhow::Error,
        /// The filesystem path where creation was attempted.
        path: String,
    },

    /// No process PID could be found for the given URL.
    #[error("PIDNotFound: Could not find the PID of an application hosting at {url}")]
    PIDNotFound {
        /// The URL whose port was scanned for a hosting process.
        url: Url,
    },

    /// Failed to read process information for the given PID.
    #[error("Could not find the application with the PID {pid}")]
    CannotReadProcessInfo {
        /// The process identifier that could not be found.
        pid: u32,
    },

    /// Failed to create an IPC or messaging channel for the application.
    #[error(
        "CannotCreateChannelForApp: Cannot create a channel for the application with url {url}"
    )]
    CannotCreateChannelForApp {
        /// The application URL for which channel creation failed.
        url: String,
    },
}

impl serde::Serialize for Error {
    /// Serializes the error into a single string (its Display representation).
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_str(self.to_string().as_ref())
    }
}
