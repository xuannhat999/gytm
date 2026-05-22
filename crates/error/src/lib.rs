use chrono::Local;
use reqwest::header::InvalidHeaderValue;
use std::{
    fs::{self, OpenOptions},
    io::Write,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum YError {
    #[error("Config File Error: {}", get_config_path_display())]
    ConfigFileError,

    #[error("IO Error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON Error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Request Error: {0}")]
    ReqwestError(#[from] reqwest::Error),

    #[error("Request Header Inval: {0}")]
    InvalidHeaderErr(#[from] InvalidHeaderValue),

    #[error("Auth Extraction Err: {0}")]
    AuthError(String),

    #[error("Tokio Task Join Err: {0}")]
    TokioJoinError(#[from] tokio::task::JoinError),

    #[error("MPV Socket Error: {0}")]
    MpvSocketError(String),

    #[error("MPV Not Running: Failed to spawn process")]
    MpvSpawnError,

    #[error("Playlist Empty: Cannot perform this action")]
    PlaylistEmpty,

    #[error("Channel Send Error: {0}")]
    ChannelSendError(String),

    #[error("Channel Recieve Error: {0}")]
    ChannelReceiveError(String),

    #[error("Invalid Cookie")]
    InvalidCookie,

    #[error("URL parsing failed: {0}")]
    UrlParseError(#[from] url::ParseError),
}
fn get_config_path_display() -> String {
    dirs::config_dir()
        .map(|p| {
            p.join("gytm")
                .join("config.json")
                .to_string_lossy()
                .into_owned()
        })
        .unwrap_or_else(|| "~/.config/gytm/config.json".to_string())
}
pub type Result<T> = std::result::Result<T, YError>;
pub fn log_to_file(message: &str) {
    if let Some(log_path) = dirs::config_dir().map(|p| p.join("gytm")) {
        let _ = fs::create_dir_all(&log_path);
        let file_path = log_path.join("log.txt");
        let datetime = Local::now().format("%Y-%m-%d %H:%M:%S");
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(file_path) {
            let _ = writeln!(file, "{} : {}", datetime, message);
        }
    }
}
