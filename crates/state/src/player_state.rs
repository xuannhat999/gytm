use crate::persist::Persist;
use data::app::PlayMode;
use serde::{Deserialize, Serialize};

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

impl Persist for PlayerState {
    const FILE_NAME: &'static str = "player_state.json";
}