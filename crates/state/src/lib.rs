use data::PlayMode;
use error::{YError, YResult};
use serde::{Deserialize, Serialize};
use std::{fs, io::Write, path::Path};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PlayerState {
    pub volume: u8,
    pub play_mode: PlayMode,
}

impl Default for PlayerState {
    fn default() -> Self {
        PlayerState {
            volume: 100,
            play_mode: PlayMode::DefaultMode,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct AppState {
    pub player_state: PlayerState,
}

impl AppState {
    pub fn load() -> YResult<Self> {
        let conf_file = Self::get_path()?;
        if !conf_file.exists() {
            return Self::create(&conf_file);
        }
        let content = fs::read_to_string(conf_file)?;
        let config: AppState = serde_json::from_str(&content)?;
        Ok(config)
    }
    pub fn save(&self) -> YResult<()> {
        let conf_file = Self::get_path()?;
        let f = fs::File::create(conf_file)?;
        serde_json::to_writer_pretty(f, self)?;
        Ok(())
    }
    pub fn create(file: &Path) -> YResult<AppState> {
        let dir = file.parent().ok_or(YError::StateFileError)?;
        fs::create_dir_all(dir)?;

        let default_config = AppState {
            player_state: PlayerState::default(),
        };
        let content = serde_json::to_string_pretty(&default_config)?;
        let mut f = fs::File::create(file)?;
        f.write_all(content.as_bytes())?;

        Ok(default_config)
    }

    pub fn get_path() -> YResult<std::path::PathBuf> {
        Ok(dirs::state_dir()
            .ok_or(YError::StateFileError)?
            .join("gytm/state.json"))
    }
}
