pub mod check;
pub mod drift;
pub mod model;
pub mod output;
pub mod render;
pub mod rust_analysis;
pub mod source;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Invalid(String),
    #[error("{path}: {source}")]
    Io {
        path: std::path::PathBuf,
        source: std::io::Error,
    },
    #[error("invalid JSON: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

pub fn io_error(path: impl AsRef<std::path::Path>, source: std::io::Error) -> Error {
    Error::Io {
        path: path.as_ref().to_owned(),
        source,
    }
}
