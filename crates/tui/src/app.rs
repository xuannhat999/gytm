use std::time::Duration;

use crate::theme::{error_style, success_style};
use data::{
    AppPage, CreatePlaylistFocus, FocusArea, NotifyType, PlayList, PlayListPrivacy, PlayMode,
    PlayerState, PlayerStatus, Song,
};
use error::log_to_file;
use ratatui::widgets::ListState;
use ratatui_notifications::{Anchor, AutoDismiss, Level, Notification, Notifications};

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

pub struct App {
    // PAGE LIBRARY
    pub albums: Vec<PlayList>,
    pub playlists: Vec<PlayList>,
    pub queue: Vec<Song>,
    pub focus_area: FocusArea,

    pub albums_liststate: ListState,
    pub playlists_liststate: ListState,
    pub queue_liststate: ListState,

    pub time_pos: Option<f64>,
    pub playing_song: Option<Song>,
    pub mpv_list: Vec<String>,

    pub playing_playlist_id: Option<String>,
    pub songs: Vec<Song>,
    pub songs_liststate: ListState,
    pub viewing_list: Option<PlayList>,

    // PAGE SEARCH
    pub search_albums: Vec<PlayList>,
    pub search_albums_liststate: ListState,
    pub search_songs: Vec<Song>,
    pub search_songs_liststate: ListState,

    pub search_query: String,
    pub is_insert: bool,

    //POPUP
    pub cus_playlists: Vec<usize>,
    pub cus_playlists_liststate: ListState,
    pub popup_state: PopupState,

    // OTHER
    pub status: PlayerStatus,
    pub volume: u8,
    pub play_mode: PlayMode,

    pub noti: Notifications,
    pub page: AppPage,
    pub is_exit: bool,
}
impl App {
    pub fn new(player_state: &PlayerState) -> Self {
        Self {
            // PAGE LIBRARY
            albums: Vec::new(),
            playlists: Vec::new(),
            queue: Vec::new(),

            albums_liststate: ListState::default(),
            playlists_liststate: ListState::default(),
            queue_liststate: ListState::default(),

            focus_area: FocusArea::Albums,

            time_pos: None,
            playing_song: None,
            songs: Vec::new(),
            songs_liststate: ListState::default(),
            mpv_list: Vec::new(),

            playing_playlist_id: None,
            viewing_list: None,

            //PAGE SEARCH
            search_albums: Vec::new(),
            search_albums_liststate: ListState::default(),
            search_songs: Vec::new(),
            search_songs_liststate: ListState::default(),
            search_query: String::new(),
            is_insert: false,

            // PLAYER
            status: PlayerStatus::Idle,
            volume: player_state.volume,
            play_mode: player_state.play_mode.clone(),

            //POPUP
            cus_playlists: Vec::new(),
            cus_playlists_liststate: ListState::default(),
            popup_state: PopupState::None,

            //OTHER
            noti: Notifications::new(),
            is_exit: false,
            page: AppPage::Library,
        }
    }
}

impl App {
    // TOGGLE NEXT ITEM IN LISTSTATE
    pub fn next_item(state: &mut ListState, len: usize) {
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
    pub fn previous_item(state: &mut ListState, len: usize) {
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
    pub fn refresh_cus_playlist(&mut self) {
        let mut new_cus: Vec<usize> = Vec::new();
        for (i, playlist) in self.playlists.iter().enumerate() {
            if playlist.is_custom {
                new_cus.push(i);
            }
        }
        self.cus_playlists = new_cus;
    }
    pub fn is_popup_active(&self) -> bool {
        !matches!(self.popup_state, PopupState::None)
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
