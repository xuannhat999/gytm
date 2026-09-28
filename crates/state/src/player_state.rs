use std::path::PathBuf;

use crate::persist::Persist;
use data::app::PlayMode;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone)]
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

impl Persist for PlayerState {
    fn file_name() -> PathBuf {
        PathBuf::from("player_state.json")
    }
}
