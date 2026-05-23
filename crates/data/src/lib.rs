use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug)]
pub struct PlayList {
    pub title: String,
    pub artist: String,
    pub browse_id: String,
    pub playlist_id: String,
}

#[derive(Deserialize, Debug)]
pub struct MpvResponse {
    pub event: Option<String>,
    pub name: Option<String>,
    #[serde(default)]
    pub data: Option<Value>,
}

#[derive(Default, Debug)]
pub struct Song {
    pub title: String,
    pub video_id: String,
}

pub enum MpvEvent {
    ListChange(Vec<String>),
    StartPlaying(Song),
    VolumeChange(u8),
}

#[derive(Default, PartialEq)]
pub enum PlayerStatus {
    #[default]
    Idle,
    Playing,
    Paused,
    Loading,
}

#[derive(Default, PartialEq, Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum PlayMode {
    #[default]
    DefaultMode,
    ShuffleMode,
}

#[derive(PartialEq)]
pub enum FocusArea {
    Albums,
    Playlists,
    SongList,
}
