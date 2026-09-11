use crate::CLIENT_STATE_FILE_NAME;
use data::client::{Browser, BrowserProfile, GeckoContainer};
use error::{YError, YResult, log_to_file};
use serde::{Deserialize, Serialize};
use std::{fs, io::Write, path::Path};

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct ClientState {
    pub browser: Option<Browser>,
    pub profile: Option<BrowserProfile>,
    pub gecko_container: Option<GeckoContainer>,
}

impl ClientState {
    pub fn load() -> YResult<Self> {
        let conf_file = Self::get_path()?;
        if !conf_file.exists() {
            return Self::create(&conf_file);
        }
        let content = fs::read_to_string(conf_file)?;
        match serde_json::from_str(&content) {
            Ok(state) => Ok(state),
            Err(e) => {
                log_to_file(&e);
                Self::create(&Self::get_path()?).ok();
                Ok(ClientState::default())
            }
        }
    }
    pub fn save(&self) -> YResult<()> {
        let state_file = Self::get_path()?;
        let f = fs::File::create(state_file)?;
        serde_json::to_writer(f, self)?;
        Ok(())
    }
    pub fn create(file: &Path) -> YResult<ClientState> {
        let dir = file
            .parent()
            .ok_or(YError::InvalidPath("~/.local/state/".to_string()))?;
        fs::create_dir_all(dir)?;
        let default_config = ClientState::default();
        let content = serde_json::to_string(&default_config)?;
        let mut f = fs::File::create(file)?;
        f.write_all(content.as_bytes())?;

        Ok(default_config)
    }

    pub fn get_path() -> YResult<std::path::PathBuf> {
        Ok(dirs::state_dir()
            .ok_or(YError::InvalidPath("~/.local/state/".to_string()))?
            .join(format!("gytm/{}", CLIENT_STATE_FILE_NAME)))
    }
}
