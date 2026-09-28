use crate::notification::NotificationManager;
use api::protocol::{ApiCmd, ApiLoadingKind};
use config::Config;
use data::app::{AppPage, FocusArea, PlayerStatus, Playlist, PopupState, SearchSongSource, Song};
use player::Player;
use ratatui::widgets::{ListState, ScrollbarState, TableState};
use state::{
    Persist, client_state::ClientState, player_state::PlayerState, queue_state::QueueState,
};
use tokio::sync::mpsc;

pub struct App {
    pub noti: NotificationManager,
    pub page: AppPage,
    pub is_exit: bool,
    pub focus_area: FocusArea,

    // ALBUMS (LIBRARY)
    pub albums: Vec<Playlist>,
    pub albums_tablestate: TableState,
    pub albums_scrollbar_state: ScrollbarState,

    // PLAYLISTS (LIBRARY)
    pub playlists: Vec<Playlist>,
    pub playlists_tablestate: TableState,
    pub playlists_scrollbar_state: ScrollbarState,

    // SONGS (CONTENT)
    pub songs: Vec<Song>,
    pub songs_tablestate: TableState,
    pub songs_scrollbar_state: ScrollbarState,

    // QUEUE
    pub queue_state: QueueState,
    pub queue_tablestate: TableState,
    pub queue_scrollbar_state: ScrollbarState,

    // PLAYER
    pub time_pos: Option<f64>,
    pub playing_song_idx: Option<usize>,
    pub mpv_list: Vec<String>,
    pub player_status: PlayerStatus,

    pub viewing_list: Option<Playlist>,

    // ALBUMS (SEARCH)
    pub search_albums: Vec<Playlist>,
    pub search_albums_tablestate: TableState,
    pub search_albums_scrollbar_state: ScrollbarState,

    // SONGS (SEARCH)
    pub search_songs: Vec<Song>,
    pub search_songs_tablestate: TableState,
    pub search_songs_scrollbar_state: ScrollbarState,

    // VIDEOS (SEARCH)
    pub search_videos: Vec<Song>,
    pub search_videos_tablestate: TableState,
    pub search_videos_scrollbar_state: ScrollbarState,

    pub search_songs_source: SearchSongSource,

    pub search_query: String,
    pub is_insert: bool,

    //POPUP
    pub popup_state: PopupState,
    pub cus_playlists: Vec<usize>,
    pub cus_playlists_liststate: ListState,

    pub browser_liststate: ListState,

    // STATE
    pub player_state: PlayerState,
    pub client_state: ClientState,
    // API WORKER
    pub api_cmd_tx: mpsc::UnboundedSender<ApiCmd>,
    pub api_loading_kind: Option<ApiLoadingKind>,
}

impl App {
    pub fn new(
        player_state: PlayerState,
        client_state: ClientState,
        queue_state: QueueState,
        config: &Config,
        api_cmd_tx: mpsc::UnboundedSender<ApiCmd>,
    ) -> Self {
        Self {
            noti: NotificationManager::new(config),
            page: AppPage::Library,
            is_exit: false,
            focus_area: FocusArea::Albums,

            // ALBUMS (LIBRARY)
            albums: Vec::new(),
            albums_tablestate: TableState::default(),
            albums_scrollbar_state: ScrollbarState::default(),

            // PLAYLISTS (LIBRARY)
            playlists: Vec::new(),
            playlists_tablestate: TableState::default(),
            playlists_scrollbar_state: ScrollbarState::default(),

            // SONGS (CONTENT)
            songs: Vec::new(),
            songs_tablestate: TableState::default(),
            songs_scrollbar_state: ScrollbarState::default(),

            // QUEUE
            queue_state,
            queue_tablestate: TableState::default(),
            queue_scrollbar_state: ScrollbarState::default(),

            // PLAYER
            time_pos: None,
            playing_song_idx: None,
            mpv_list: Vec::new(),
            player_status: PlayerStatus::Idle,

            viewing_list: None,

            // ALBUMS (SEARCH)
            search_albums: Vec::new(),
            search_albums_tablestate: TableState::default(),
            search_albums_scrollbar_state: ScrollbarState::default(),

            // SONGS (SEARCH)
            search_songs: Vec::new(),
            search_songs_tablestate: TableState::default(),
            search_songs_scrollbar_state: ScrollbarState::default(),

            // VIDEOS (SEARCH)
            search_videos: Vec::new(),
            search_videos_tablestate: TableState::default(),
            search_videos_scrollbar_state: ScrollbarState::default(),

            search_songs_source: SearchSongSource::default(),

            search_query: String::new(),
            is_insert: false,

            //POPUP
            popup_state: PopupState::None,
            cus_playlists: Vec::new(),
            cus_playlists_liststate: ListState::default(),

            browser_liststate: ListState::default(),

            // STATE
            player_state,
            client_state,
            // API WORKER
            api_cmd_tx,
            api_loading_kind: None,
        }
    }

    pub fn get_search_songs_from_source(&self) -> &[Song] {
        match self.search_songs_source {
            SearchSongSource::Song => &self.search_songs,
            SearchSongSource::Video => &self.search_videos,
        }
    }

    pub fn get_search_songs_states_from_source(&mut self) -> &mut TableState {
        match self.search_songs_source {
            SearchSongSource::Song => &mut self.search_songs_tablestate,
            SearchSongSource::Video => &mut self.search_videos_tablestate,
        }
    }
    pub fn selected_search_song(&self) -> Option<&Song> {
        let (songs, state) = match self.search_songs_source {
            SearchSongSource::Song => (&self.search_songs, &self.search_songs_tablestate),
            SearchSongSource::Video => (&self.search_videos, &self.search_videos_tablestate),
        };
        state.selected().and_then(|i| songs.get(i))
    }

    pub fn toggle_search_songs_source(&mut self) {
        self.search_songs_source = match self.search_songs_source {
            SearchSongSource::Song => SearchSongSource::Video,
            SearchSongSource::Video => SearchSongSource::Song,
        }
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

    pub fn shutdown(&self, player: &mut Player) {
        QueueState::delete().ok();
        player.shutdown();
    }
}
