use ::serde::Deserialize;
use error::{Result, YError};
use serde::Serialize;
use std::{fs, io::Write, path::Path};

#[derive(Debug, Deserialize, Serialize)]
pub struct AppConfig {
    pub user_agent: String,
}

impl AppConfig {
    pub fn load() -> Result<Self> {
        let conf_file = dirs::config_dir()
            .ok_or(YError::ConfigFileError)?
            .join("gytm/config.json");

        if !conf_file.exists() {
            return create_config_file(&conf_file);
        }
        let content = fs::read_to_string(conf_file)?;
        let config: AppConfig = serde_json::from_str(&content)?;
        Ok(config)
    }
}

#[derive(Debug)]
pub struct PlayList {
    pub title: String,
    pub artist: String,
    pub browse_id: String,
    pub playlist_id: String,
}

#[derive(Default, Debug)]
pub struct Song {
    pub title: String,
    pub video_id: String,
}

pub fn create_config_file(file: &Path) -> Result<AppConfig> {
    let dir = file.parent().ok_or(YError::ConfigFileError)?;
    fs::create_dir_all(dir)?;

    let default_config = AppConfig {
        user_agent : "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36".to_string(),
    };
    let content = serde_json::to_string_pretty(&default_config)?;
    let mut f = fs::File::create(file)?;
    f.write_all(serde_json::to_string_pretty(&content)?.as_bytes())?;

    Ok(default_config)
}
