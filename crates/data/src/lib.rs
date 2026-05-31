use ratatui::style::{Color, Modifier, Style};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct PlayList {
    pub title: String,
    pub artist: String,
    pub browse_id: String,
    pub playlist_id: String,
    pub is_saved: bool,
}
#[derive(Default, Debug, Clone)]
pub struct Song {
    pub title: String,
    pub video_id: String,
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
    SearchAlbums,
    SearchSongs,
}

#[derive(PartialEq)]
pub enum AppPage {
    Library,
    Search,
}

pub struct Theme {
    pub primary: Color,
    pub secondary: Color,
    pub active: Color,
    pub inactive: Color,
    pub base: Color,
}
impl Default for Theme {
    fn default() -> Self {
        Theme {
            primary: Color::LightGreen,
            secondary: Color::LightYellow,
            active: Color::LightCyan,
            inactive: Color::DarkGray,
            base: Color::White,
        }
    }
}
impl Theme {
    pub fn text_style(&self) -> Style {
        Style::default().fg(self.base)
    }

    pub fn key_style(&self) -> Style {
        Style::default()
            .fg(self.secondary)
            .add_modifier(Modifier::BOLD)
    }
    pub fn active_border_style(&self) -> Style {
        Style::default()
            .fg(self.active)
            .add_modifier(Modifier::BOLD)
    }
    pub fn inactive_border_style(&self) -> Style {
        Style::default().fg(self.inactive)
    }
    pub fn selected_item(&self) -> Style {
        Style::default().bg(Color::Rgb(69, 71, 90)).fg(self.primary)
    }
}
