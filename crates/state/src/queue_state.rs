use std::path::PathBuf;

use data::app::Song;
use serde::{Deserialize, Serialize};

use crate::Persist;

#[derive(Default, Deserialize, Serialize)]
pub struct QueueState {
    pub playing_playlist_id: Option<String>,
    pub queue: Vec<Song>,
}

impl Persist for QueueState {
    fn file_name() -> std::path::PathBuf {
        PathBuf::from("queue_state.json")
    }
}
