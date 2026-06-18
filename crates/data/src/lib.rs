use serde::{Deserialize, Serialize};
use serde_json::Value;
pub mod file_path;

#[derive(Debug, Clone)]
pub struct PlayList {
    pub title: String,
    pub artist: String,
    pub browse_id: String,
    pub playlist_id: String,
    pub is_saved: bool,
    pub is_custom: bool,
}
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Song {
    pub title: String,
    pub set_video_id: String,
    pub video_id: String,
    pub duration: String,
}

#[derive(Deserialize, Debug)]
pub struct MpvResponse {
    pub event: Option<String>,
    pub name: Option<String>,
    pub data: Option<Value>,
}

pub enum MpvEvent {
    ListChange(Vec<String>),
    StartPlaying(String),
    VolumeChange(u8),
    TimePos(f64),
    PauseChange(bool),
}

#[derive(PartialEq)]
pub enum MpvCommand {
    Shuffle,
    Unshuffle,
    SeekForward,
    SeekBackward,
    PlayNext,
    PlayPrev,
    TogglePause,
    IncreaseVol,
    DecreaseVol,
    SetVol(u8),
    PlayPos(usize),
    AppendSong(String),
    LoadList,
    RemovePos(usize),
    Stop,
    Clear,
    Quit,
}
#[derive(PartialEq)]
pub enum PlayerStatus {
    Idle,
    Playing,
    Paused,
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
    Songs,
}

#[derive(PartialEq)]
pub enum CreatePlaylistFocus {
    Title,
    Description,
    Privacy,
}

#[derive(PartialEq, Copy, Clone)]
pub enum AppPage {
    Library = 0,
    Search = 1,
}

pub enum NotifyType {
    Success,
    Error,
}

#[derive(Serialize, PartialEq, Copy, Clone)]
pub enum PlayListPrivacy {
    Private,
    Public,
    Unlisted,
}

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
