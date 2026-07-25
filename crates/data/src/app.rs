use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct Playlist {
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
    #[serde(default)]
    pub artist: String,
    pub set_video_id: String,
    pub video_id: String,
    pub duration: String,
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

#[derive(Serialize, PartialEq, Copy, Clone)]
pub enum PlayListPrivacy {
    Private,
    Public,
    Unlisted,
}

pub enum PopupState {
    None,
    SaveSong {
        selected_save_song: Song,
    },
    CreatePlaylist {
        title: String,
        description: String,
        privacy: PlayListPrivacy,
        focused_field: CreatePlaylistFocus,
    },
}
