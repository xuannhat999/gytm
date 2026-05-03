use data::{PlayList, Song};
use player::{self, MusicPlayer};
use ratatui::widgets::ListState;

#[derive(PartialEq)]
pub enum FocusArea {
    Albums,
    Playlists,
    SongList,
}
pub struct App {
    pub player: MusicPlayer,

    pub albums: Vec<PlayList>,
    pub playlists: Vec<PlayList>,
    pub songs: Vec<Song>,
    pub focus_area: FocusArea,

    pub album_list_state: ListState,
    pub playlist_list_state: ListState,
    pub songs_list_state: ListState,

    pub is_loading: bool,
    pub is_exit: bool,
}
impl App {
    pub fn new() -> Self {
        Self {
            player: MusicPlayer::default(),
            is_exit: false,
            albums: Vec::new(),
            playlists: Vec::new(),
            songs: Vec::new(),
            album_list_state: ListState::default(),
            playlist_list_state: ListState::default(),
            songs_list_state: ListState::default(),
            focus_area: FocusArea::Albums,
            is_loading: false,
        }
    }

    // FOCUS NEXT AREA
    pub fn toggle_focus(&mut self) {
        self.focus_area = match self.focus_area {
            FocusArea::Albums => FocusArea::Playlists,
            FocusArea::Playlists => FocusArea::Albums,
            FocusArea::SongList => FocusArea::SongList,
        };
    }

    // TOGGLE NEXT ITEM IN LISTSTATE
    pub fn next(&mut self) {
        let (state, len) = match self.focus_area {
            FocusArea::Albums => (&mut self.album_list_state, self.albums.len()),
            FocusArea::Playlists => (&mut self.playlist_list_state, self.playlists.len()),
            FocusArea::SongList => (&mut self.songs_list_state, self.songs.len()),
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
            FocusArea::Albums => (&mut self.album_list_state, self.albums.len()),
            FocusArea::Playlists => (&mut self.playlist_list_state, self.playlists.len()),
            FocusArea::SongList => (&mut self.songs_list_state, self.songs.len()),
        };

        if len == 0 {
            return;
        }

        let i = match state.selected() {
            Some(i) => {
                if i == 0 {
                    len - 1 // Quay xuống cuối danh sách
                } else {
                    i - 1
                }
            }
            None => 0,
        };

        state.select(Some(i));
    }
}
