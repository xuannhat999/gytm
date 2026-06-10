use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct PlayList {
    pub title: String,
    pub artist: String,
    pub browse_id: String,
    pub playlist_id: String,
    pub is_saved: bool,
    pub is_custom: bool,
}
#[derive(Default, Debug, Clone)]
pub struct Song {
    pub title: String,
    pub video_id: String,
    pub duration: String,
}

#[derive(Deserialize, Debug)]
pub struct MpvResponse {
    pub event: Option<String>,
    pub name: Option<String>,
    #[serde(default)]
    pub data: Option<Value>,
}

pub enum MpvEvent {
    ListChange(Vec<String>),
    StartPlaying(String),
    VolumeChange(u8),
    TimePos(f64),
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
    Queue,
    SearchAlbums,
    SearchSongs,
}

#[derive(PartialEq, Copy, Clone)]
pub enum AppPage {
    Library = 0,
    Search = 1,
}
