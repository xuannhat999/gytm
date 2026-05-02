use thiserror::Error;

#[derive(Debug, Error)]
pub enum YError {
    #[error("Config File Err")]
    ConfigFileErr,

    #[error("Read File Err: {0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON Format Err: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Request Err: {0}")]
    ReqwestError(#[from] reqwest::Error),

    #[error("Request Header Inval: {0}")]
    InvalidHeaderErr(#[from] reqwest::header::InvalidHeaderValue),

    #[error("Auth Extraction Err: {0}")]
    AuthError(String),

    #[error("Tokio Task Join Err: {0}")]
    TokioJoinError(#[from] tokio::task::JoinError),
}

pub type Result<T> = std::result::Result<T, YError>;
