use crate::{CLIENT_STATE_FILE_NAME, player_state::PlayerState};
use data::client::{Browser, BrowserProfile, GeckoContainer};
use error::{YError, YResult, log_to_file};
use serde::{Deserialize, Serialize};
use std::{fs, io::Write, path::PathBuf};

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct ClientState {
    pub browser: Option<Browser>,
    pub profile: Option<BrowserProfile>,
    pub gecko_container: Option<GeckoContainer>,
}

impl ClientState {
    pub fn load() -> YResult<Self> {
        if let Some(player_state_filepath) = Self::get_path() {
            if player_state_filepath.exists() && player_state_filepath.is_file() {
                let content = fs::read_to_string(&player_state_filepath)?;
                match serde_json::from_str(&content) {
                    Ok(state) => return Ok(state),
                    Err(e) => {
                        log_to_file(&e);
                    }
                }
            } else {
                log_to_file("Player state file doesn't exists");
            }
        }
        Self::create()
    }

    pub fn save(&self) -> YResult<()> {
        if let Some(state_file) = Self::get_path() {
            let f = fs::File::create(&state_file)?;
            serde_json::to_writer(f, self)?;
        }
        Ok(())
    }
    pub fn create() -> YResult<ClientState> {
        if let Some(state_dir) = dirs::state_dir() {
            let base_dir = state_dir.join("gytm");
            fs::create_dir_all(&base_dir)?;
            let default_config = ClientState::default();
            let content = serde_json::to_string(&default_config)?;
            let file = base_dir.join(CLIENT_STATE_FILE_NAME);
            let mut f = fs::File::create(file)?;
            f.write_all(content.as_bytes())?;
            return Ok(default_config);
        }
        Err(YError::InvalidPath("STATE DIR".to_string()))
    }

    pub fn get_path() -> Option<PathBuf> {
        dirs::state_dir().map(|p| p.join(format!("gytm/{}", CLIENT_STATE_FILE_NAME)))
    }
}
