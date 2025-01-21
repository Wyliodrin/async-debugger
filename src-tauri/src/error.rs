#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("URL: {0}")]
    Url(#[from] url::ParseError),
}

impl serde::Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_str(self.to_string().as_ref())
    }
}
