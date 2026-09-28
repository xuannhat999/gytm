use ratatui::widgets::ListState;
use serde::{Deserialize, Serialize};

use crate::api_client::{Account, Browser, BrowserProfile, GeckoContainer};

#[derive(Clone)]
pub struct Playlist {
    pub title: String,
    pub artist: String,
    pub browse_id: String,
    pub playlist_id: String,
    pub is_saved: bool,
    pub is_custom: bool,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Song {
    pub title: String,
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

#[derive(Default)]
pub enum SearchSongSource {
    #[default]
    Song,
    Video,
}

#[derive(Default, PartialEq, Serialize, Deserialize, Clone)]
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
        custom_playlists_idx: Vec<usize>,
        custom_playlists_liststate: ListState,
    },
    CreatePlaylist {
        title: String,
        description: String,
        privacy: PlayListPrivacy,
        focused_field: CreatePlaylistFocus,
    },
    ApiCLient,
    SelectBrowser {
        browsers_liststate: ListState,
    },
    SelectBrowserProfile {
        browser: Browser,
        profiles: Vec<BrowserProfile>,
        profiles_liststate: ListState,
    },
    SelectGeckoContainer {
        browser: Browser,
        profile: BrowserProfile,
        containers: Vec<GeckoContainer>,
        containers_liststate: ListState,
    },
    SelectAccount {
        browser: Browser,
        profile: BrowserProfile,
        container: Option<GeckoContainer>,
        accounts: Vec<Account>,
        accounts_liststate: ListState,
    },
}
