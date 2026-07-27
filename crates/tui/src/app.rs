use crate::{
    helper::{self, get_queue_file},
    notification::NotificationManager,
};
use api::protocol::{ApiCmd, ApiLoadingKind};
use config::Config;
use data::app::{
    AppPage, FocusArea, PlayMode, PlayerStatus, Playlist, PopupState, QueueData, Song,
};
use error::YResult;
use player::Player;
use ratatui::widgets::ListState;
use state::PlayerState;
use tokio::sync::mpsc;

pub struct App {
    // PAGE LIBRARY
    pub albums: Vec<Playlist>,
    pub playlists: Vec<Playlist>,
    pub queue: Vec<Song>,
    pub focus_area: FocusArea,

    pub albums_liststate: ListState,
    pub playlists_liststate: ListState,
    pub queue_liststate: ListState,

    pub time_pos: Option<f64>,
    pub playing_song: Option<usize>,
    pub mpv_list: Vec<String>,

    pub playing_playlist_id: Option<String>,
    pub songs: Vec<Song>,
    pub songs_liststate: ListState,
    pub viewing_list: Option<Playlist>,

    // PAGE SEARCH
    pub search_albums: Vec<Playlist>,
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

    pub noti: NotificationManager,
    pub page: AppPage,
    pub is_exit: bool,

    // API WORKER
    pub api_cmd_tx: mpsc::UnboundedSender<ApiCmd>,
    pub api_loading_kind: Option<ApiLoadingKind>,
}

impl App {
    pub fn new(
        player_state: &PlayerState,
        config: &Config,
        api_cmd_tx: mpsc::UnboundedSender<ApiCmd>,
    ) -> Self {
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
            noti: NotificationManager::new(config),
            is_exit: false,
            page: AppPage::Library,

            // API WORKER
            api_cmd_tx,
            api_loading_kind: None,
        }
    }

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

    pub fn save_queue_file(&self) -> YResult<()> {
        let path = helper::get_queue_file()?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let data = QueueData {
            queue: self.queue.clone(),
            playing_playlist_id: self.playing_playlist_id.clone(),
        };
        let content = serde_json::to_string(&data)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    pub fn load_queue_file(&mut self) -> YResult<()> {
        let path = get_queue_file()?;
        if path.exists() {
            let content = std::fs::read_to_string(path)?;
            let data: QueueData = serde_json::from_str(&content).unwrap_or_default();
            self.queue = data.queue;
            self.playing_playlist_id = data.playing_playlist_id;
        }
        Ok(())
    }

    pub fn shutdown(player: &mut Player) {
        helper::remove_queue_file();
        player.shutdown();
    }
}
