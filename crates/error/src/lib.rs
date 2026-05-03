use thiserror::Error;

#[derive(Debug, Error)]
pub enum YError {
    #[error("Config File Error")]
    ConfigFileErr,

    #[error("Read File Error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON Format Error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Request Error: {0}")]
    ReqwestError(#[from] reqwest::Error),

    #[error("Request Header Inval: {0}")]
    InvalidHeaderErr(#[from] reqwest::header::InvalidHeaderValue),

    #[error("Auth Extraction Err: {0}")]
    AuthError(String),

    #[error("Tokio Task Join Err: {0}")]
    TokioJoinError(#[from] tokio::task::JoinError),

    #[error(
        "Invalid Cookie: Please fill your cookie in {} and restart",
        get_config_path_display()
    )]
    InvalidCookie,
}
fn get_config_path_display() -> String {
    dirs::config_dir()
        .map(|p| {
            p.join("ytm")
                .join("config.json")
                .to_string_lossy()
                .into_owned()
        })
        .unwrap_or_else(|| "~/.config/ytm/config.json".to_string())
}
pub type Result<T> = std::result::Result<T, YError>;
