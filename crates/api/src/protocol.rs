use data::{PlayListPrivacy, Playlist, Song};
use error::YResult;

pub enum ApiCmd {
    CreatePlaylist {
        title: String,
        description: String,
        privacy: PlayListPrivacy,
    },
    SaveSong {
        song: Song,
        playlist_id: String,
    },
    UnsaveSong {
        song: Song,
        playlist_id: String,
    },
    Search(String),
    LikeSong(Song),
    UnlikeSong(Song),
    GetSongsToView(Playlist),
    GetSongsToPlay(Playlist),
    UnsaveAlbum(Playlist),
    UnsaveCusPlaylist(Playlist),
    SaveAlbum(Playlist),
    GetRelatedSongsToPlay(Song),
    FetchLibraryData,
}

pub enum ApiResponse {
    CreatePlaylist(YResult<Playlist>),
    SaveSong(YResult<(Song, String)>),
    UnsaveSong((YResult<()>, String)),
    LikeSong(YResult<Song>),
    UnlikeSong((YResult<()>, String)),
    Search {
        albums: YResult<Vec<Playlist>>,
        songs: YResult<Vec<Song>>,
    },
    GetSongsToView {
        songs: YResult<Vec<Song>>,
        playlist: Playlist,
    },
    GetSongsToPlay {
        songs: YResult<Vec<Song>>,
        playlist_id: String,
    },
    UnsaveAlbum((YResult<()>, Playlist)),
    UnsaveCusPlaylist((YResult<()>, String)),
    SaveAlbum((YResult<()>, Playlist)),
    GetRelatedSongsToPlay(YResult<Vec<Song>>),
    FetchLibraryData(YResult<(Vec<Playlist>, Vec<Playlist>, Vec<usize>)>),
}

#[derive(PartialEq)]
pub enum ApiLoadingKind {
    CreatePlaylist,
    SaveToPlaylist,
    Search,
    FetchLibraryData,
    GetSongsToView,
    GetSongsToPlay,
}
