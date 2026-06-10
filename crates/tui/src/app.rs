use std::time::Duration;

use data::{AppPage, FocusArea, NotifyType, PlayList, Song};
use error::log_to_file;
use ratatui::{style::Style, widgets::ListState};
use ratatui_notifications::{Anchor, AutoDismiss, Level, Notification, Notifications};

pub struct App {
    // PAGE LIBRARY
    pub albums: Vec<PlayList>,
    pub playlists: Vec<PlayList>,
    pub songs: Vec<Song>,
    pub focus_area: FocusArea,

    pub albums_liststate: ListState,
    pub playlists_liststate: ListState,
    pub songs_liststate: ListState,

    pub time_pos: Option<f64>,
    pub playing_song: Option<Song>,
    pub mpv_list: Vec<String>,

    pub playing_playlist_id: Option<String>,

    // PAGE SEARCH
    pub search_albums: Vec<PlayList>,
    pub search_albums_liststate: ListState,
    pub search_songs: Vec<Song>,
    pub search_songs_liststate: ListState,

    pub search_query: String,
    pub is_insert: bool,

    // OTHER
    pub noti: Notifications,
    pub page: AppPage,
    pub is_exit: bool,
}
impl Default for App {
    fn default() -> Self {
        Self {
            // PAGE LIBRARY
            albums: Vec::new(),
            playlists: Vec::new(),
            songs: Vec::new(),

            albums_liststate: ListState::default(),
            playlists_liststate: ListState::default(),
            songs_liststate: ListState::default(),

            focus_area: FocusArea::Albums,

            time_pos: None,
            playing_song: None,

            mpv_list: Vec::new(),

            playing_playlist_id: None,

            //PAGE SEARCH
            search_albums: Vec::new(),
            search_albums_liststate: ListState::default(),
            search_songs: Vec::new(),
            search_songs_liststate: ListState::default(),
            search_query: String::new(),
            is_insert: false,

            //OTHER
            noti: Notifications::new(),
            is_exit: false,
            page: AppPage::Library,
        }
    }
}

impl App {
    // TOGGLE NEXT ITEM IN LISTSTATE
    pub fn next(&mut self) {
        let (state, len) = match self.focus_area {
            FocusArea::Albums => (&mut self.albums_liststate, self.albums.len()),
            FocusArea::Playlists => (&mut self.playlists_liststate, self.playlists.len()),
            FocusArea::Queue => (&mut self.songs_liststate, self.songs.len()),
            FocusArea::SearchAlbums => {
                (&mut self.search_albums_liststate, self.search_albums.len())
            }
            FocusArea::SearchSongs => (&mut self.search_songs_liststate, self.search_songs.len()),
        };

        if len == 0 {
            return;
        }

        let i = match state.selected() {
            Some(i) => {
                if i >= len - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };

        state.select(Some(i));
    }

    // TOGGLE PREVIOUS ITEM IN LISTSTATE
    pub fn previous(&mut self) {
        let (state, len) = match self.focus_area {
            FocusArea::Albums => (&mut self.albums_liststate, self.albums.len()),
            FocusArea::Playlists => (&mut self.playlists_liststate, self.playlists.len()),
            FocusArea::Queue => (&mut self.songs_liststate, self.songs.len()),
            FocusArea::SearchAlbums => {
                (&mut self.search_albums_liststate, self.search_albums.len())
            }
            FocusArea::SearchSongs => (&mut self.search_songs_liststate, self.search_songs.len()),
        };

        if len == 0 {
            return;
        }

        let i = match state.selected() {
            Some(i) => {
                if i == 0 {
                    len - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        state.select(Some(i));
    }
    pub fn get_mpv_idx(&self, id: &str) -> Option<usize> {
        for (pos, mpv_id) in self.mpv_list.iter().enumerate() {
            if id == mpv_id {
                return Some(pos);
            }
        }
        None
    }
    pub fn notify(&mut self, noti_type: NotifyType, msg: String) {
        let style = match noti_type {
            NotifyType::Error => error_style(),
            NotifyType::Success => success_style(),
        };
        if let Ok(notif) = Notification::new(msg)
            .level(Level::Info)
            .anchor(Anchor::TopRight)
            .auto_dismiss(AutoDismiss::After(Duration::from_millis(1600)))
            .max_size(
                ratatui_notifications::SizeConstraint::Percentage(25.0),
                ratatui_notifications::SizeConstraint::Absolute(4),
            )
            .timing(
                ratatui_notifications::Timing::Fixed(Duration::from_millis(50)),
                ratatui_notifications::Timing::Fixed(Duration::from_millis(1500)),
                ratatui_notifications::Timing::Fixed(Duration::from_millis(50)),
            )
            .style(style)
            .border_style(style)
            .build()
        {
            if let Err(e) = self.noti.add(notif) {
                log_to_file(&e);
            }
        }
    }
}
fn success_style() -> Style {
    Style::default().fg(ratatui::style::Color::Green)
}
fn error_style() -> Style {
    Style::default().fg(ratatui::style::Color::Red)
}
