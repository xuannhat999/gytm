use crate::Persist;
use data::app::Song;
use serde::{Deserialize, Serialize};

#[derive(Default, Deserialize, Serialize)]
pub struct QueueState {
    pub playing_playlist_id: Option<String>,
    pub queue: Vec<Song>,
}

impl Persist for QueueState {
    const FILE_NAME: &'static str = "queue_state.json";
}
