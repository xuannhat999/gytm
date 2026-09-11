use data::app::PlayMode;
use error::{YError, YResult, log_to_file};
use serde::{Deserialize, Serialize};
use std::{fs, io::Write, path::PathBuf};

use crate::PLAYER_STATE_FILE_NAME;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PlayerState {
    pub volume: u8,
    pub play_mode: PlayMode,
}

impl Default for PlayerState {
    fn default() -> Self {
        Self {
            volume: 100,
            play_mode: PlayMode::DefaultMode,
        }
    }
}

impl PlayerState {
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
    pub fn create() -> YResult<PlayerState> {
        if let Some(state_dir) = dirs::state_dir() {
            let base_dir = state_dir.join("gytm");
            fs::create_dir_all(&base_dir)?;
            let default_config = PlayerState::default();
            let content = serde_json::to_string(&default_config)?;
            let file = base_dir.join(PLAYER_STATE_FILE_NAME);
            let mut f = fs::File::create(file)?;
            f.write_all(content.as_bytes())?;
            return Ok(default_config);
        }
        Err(YError::InvalidPath("STATE DIR".to_string()))
    }

    pub fn get_path() -> Option<PathBuf> {
        dirs::state_dir().map(|p| p.join(format!("gytm/{}", PLAYER_STATE_FILE_NAME)))
    }
}
