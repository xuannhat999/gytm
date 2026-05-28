use reqwest::header::InvalidHeaderValue;
use std::{
    fmt::{Debug, Display},
    fs::{self, OpenOptions},
    io::Write,
};
use thiserror::Error;
use time::{OffsetDateTime, format_description};

#[derive(Debug, Error)]
pub enum YError {
    #[error("Config Dir Error")]
    ConfigDirError,

    #[error("State File Error")]
    StateFileError,

    #[error("Invalid File Path")]
    InvalidFilePath,

    #[error("IO Error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON Error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Request Error: {0}")]
    ReqwestError(#[from] reqwest::Error),

    #[error("Request Header Inval: {0}")]
    InvalidHeader(#[from] InvalidHeaderValue),

    // #[error("Auth Extraction Err: {0}")]
    // AuthError(String),
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

pub type YResult<T> = std::result::Result<T, YError>;

pub fn log_to_file<T: Display>(message: T) {
    if let Some(log_path) = dirs::state_dir().map(|p| p.join("gytm")) {
        if !log_path.exists() {
            let _ = fs::create_dir_all(&log_path);
        }
        let file_path = log_path.join("log.txt");

        let now = OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc());

        let format =
            format_description::parse("[year]-[month]-[day] [hour]:[minute]:[second]").unwrap();

        let datetime = now.format(&format).unwrap_or_default();

        let max_size = 5 * 1024 * 1024;
        let is_oversize = fs::metadata(&file_path)
            .map(|meta| meta.len() >= max_size)
            .unwrap_or(false);

        let mut options = OpenOptions::new();
        options.create(true).write(true);

        if is_oversize {
            options.truncate(true);
        } else {
            options.append(true);
        }
        if let Ok(mut file) = options.open(file_path) {
            let _ = writeln!(file, "{} : {}", datetime, message);
        }
    }
}
