use data::mpv::{MpvCommand, MpvEvent};
use reqwest::header::InvalidHeaderValue;
use std::{
    fmt::{Debug, Display},
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
    sync::{
        OnceLock,
        atomic::{AtomicU64, Ordering},
    },
};
use thiserror::Error;
use time::{OffsetDateTime, format_description, format_description::BorrowedFormatItem};

#[derive(Debug, Error)]
pub enum YError {
    #[error("Path doesn't exist: {0}")]
    InvalidPath(String),

    #[error("Invalid Response from: {0}")]
    InvalidResponse(String),

    #[error("IO Error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON Error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Request Error: {0}")]
    ReqwestError(#[from] reqwest::Error),

    #[error("Request Header Inval: {0}")]
    InvalidHeader(#[from] InvalidHeaderValue),

    #[error("MPV Socket Error: {0}")]
    MpvSocketError(String),

    #[error("Invalid Cookie")]
    InvalidCookie,

    #[error("Cookie expired")]
    CookieExpired,

    #[error("Unavailable feature for guest mode")]
    UnavailableFeature,

    #[error("Miss API client context: {0}")]
    MissApiClientContext(String),

    #[error("Sqlite Error: {0}")]
    SqliteError(#[from] rusqlite::Error),

    #[error("Rust Ini Error: {0}")]
    RustIni(#[from] ini::Error),

    #[error("URL parsing failed: {0}")]
    UrlParseError(#[from] url::ParseError),

    #[error("Event Sender Error: {0}")]
    EventSenderError(#[from] tokio::sync::mpsc::error::SendError<MpvEvent>),

    #[error("Command Sender Error: {0}")]
    MpvCmdSenderError(#[from] tokio::sync::mpsc::error::SendError<MpvCommand>),

    #[error("Song alredy saved in playlist")]
    AlreadyInPlaylist,

    #[error("Invalid file content in: {0}")]
    InvalidFileContent(PathBuf),

    #[error("Extract chromium cookie failed: {0}")]
    RookieError(String),

    #[error("Bad Status from: {0}")]
    BadStatus(String),
}

pub type YResult<T> = std::result::Result<T, YError>;

const LOG_MAX_SIZE: u64 = 5 * 1024 * 1024;

static LOG_FORMAT: OnceLock<Vec<BorrowedFormatItem<'static>>> = OnceLock::new();
static LOG_FILE: OnceLock<Option<PathBuf>> = OnceLock::new();
static LOG_SIZE: AtomicU64 = AtomicU64::new(0);

fn log_format() -> &'static Vec<BorrowedFormatItem<'static>> {
    LOG_FORMAT.get_or_init(|| {
        format_description::parse("[year]-[month]-[day] [hour]:[minute]:[second]")
            .expect("static log timestamp format")
    })
}

fn log_file_path() -> Option<PathBuf> {
    LOG_FILE
        .get_or_init(|| {
            dirs::state_dir().map(|p| {
                let dir = p.join("gytm");
                let _ = fs::create_dir_all(&dir);
                let path = dir.join("gytm.log");
                let size = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                LOG_SIZE.store(size, Ordering::Relaxed);
                path
            })
        })
        .clone()
}

pub fn log_to_file<T: Display>(message: T) {
    let Some(file_path) = log_file_path() else {
        return;
    };

    let now = OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc());
    let datetime = now.format(log_format()).unwrap_or_default();
    let line = format!("{} : {}\n", datetime, message);

    if LOG_SIZE.load(Ordering::Relaxed) >= LOG_MAX_SIZE {
        let backup_path = file_path.with_extension("log.bak");
        let _ = fs::rename(&file_path, &backup_path);
        LOG_SIZE.store(0, Ordering::Relaxed);
    }

    let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&file_path)
    else {
        return;
    };

    if file.write_all(line.as_bytes()).is_ok() {
        LOG_SIZE.fetch_add(line.len() as u64, Ordering::Relaxed);
    }
}
