use data::{PlayListPrivacy, Playlist, Song};
use error::{YResult, log_to_file};

use crate::{dao::YTDao, parser};

pub struct YTBus {
    dao: YTDao,
}

impl YTBus {
    pub fn new(dao: YTDao) -> Self {
        Self { dao }
    }

    pub async fn create_playlist(
        &self,
        title: &str,
        desc: &str,
        privacy: PlayListPrivacy,
    ) -> YResult<Playlist> {
        let res = self.dao.create_playlist_raw(title, desc, privacy).await?;
        let playlist = parser::parse_created_playlist(&res)?;
        Ok(playlist)
    }

    pub async fn get_lists(&self) -> YResult<(Vec<Playlist>, Vec<Playlist>, Vec<usize>)> {
        let mut all_albums: Vec<Playlist> = Vec::new();
        let mut all_playlists: Vec<Playlist> = Vec::new();
        let mut all_cus_playlists: Vec<usize> = Vec::new();
        let raw_data = self.dao.get_raw_lists().await?;

        let (mut albums, mut playlists, mut token) = match parser::parse_lists(&raw_data) {
            Ok((albums, playlists, token)) => (albums, playlists, token),
            Err(e) => {
                log_to_file(&e);
                return Err(e);
            }
        };
        all_albums.append(&mut albums);
        all_playlists.append(&mut playlists);

        while let Some(current_token) = token {
            let next_raw_data = self.dao.get_continuation_raw(&current_token).await?;
            let (mut next_albums, mut next_playlists, next_token) =
                parser::parse_lists(&next_raw_data)?;
            all_albums.append(&mut next_albums);
            all_playlists.append(&mut next_playlists);
            token = next_token;
        }
        for (idx, playlist) in all_playlists.iter_mut().enumerate() {
            if playlist.playlist_id == "LM" {
                playlist.is_custom = true;
            }
            if playlist.is_custom {
                all_cus_playlists.push(idx);
            }
        }
        Ok((all_albums, all_playlists, all_cus_playlists))
    }

    pub async fn get_songs(&self, browse_id: &str) -> YResult<Vec<Song>> {
        let raw = self.dao.get_songs_raw(browse_id).await?;
        match parser::parse_songs(&raw) {
            Ok(songs) => Ok(songs),
            Err(e) => {
                log_to_file(&e);
                Err(e)
            }
        }
    }

    pub async fn get_search_albums(&self, query: &str) -> YResult<Vec<Playlist>> {
        let raw_list = self.dao.get_search_albums_raw(query, 2).await?;
        match parser::parse_search_albums(&raw_list) {
            Ok(list) => Ok(list),
            Err(e) => {
                log_to_file(&e);
                Err(e)
            }
        }
    }

    pub async fn get_search_songs(&self, query: &str) -> YResult<Vec<Song>> {
        let raw_data = self.dao.get_search_albums_raw(query, 1).await?;
        match parser::parse_search_songs(&raw_data) {
            Ok(songs) => Ok(songs),
            Err(e) => {
                log_to_file(&e);
                Err(e)
            }
        }
    }

    pub async fn get_params(&self, video_id: &str) -> YResult<String> {
        let raw_data = self.dao.get_params_raw(video_id).await?;
        match parser::parse_params(&raw_data) {
            Ok(params) => Ok(params),
            Err(e) => {
                log_to_file(&e);
                Err(e)
            }
        }
    }

    pub async fn get_related_songs(&self, song: Song, params: &str) -> YResult<Vec<Song>> {
        let video_id = &song.video_id;
        let playlist_id = format!("RDAMVM{}", video_id);
        let raw_data = self.dao.get_related_songs_raw(&playlist_id, params).await?;
        let mut songs = match parser::parse_related_songs(&raw_data) {
            Ok(songs) => songs,
            Err(e) => {
                log_to_file(&e);
                Vec::new()
            }
        };
        songs.insert(0, song);
        Ok(songs)
    }

    pub async fn save_to_playlist(&self, song: &Song, playlist_id: &str) -> YResult<()> {
        self.dao.save_to_playlist_raw(song, playlist_id).await
    }

    pub async fn unsave_to_playlist(&self, song: &Song, playlist_id: &str) -> YResult<()> {
        self.dao.unsave_to_playlist_raw(song, playlist_id).await
    }

    pub async fn like_song(&self, song: &Song) -> YResult<()> {
        self.dao.like_song_raw(song).await
    }

    pub async fn unlike_song(&self, song: &Song) -> YResult<()> {
        self.dao.unlike_song_raw(song).await
    }

    pub async fn save_album(&self, playlist_id: &str) -> YResult<()> {
        self.dao.save_album_raw(playlist_id).await
    }

    pub async fn unsave_album(&self, playlist_id: &str) -> YResult<()> {
        self.dao.unsave_album_raw(playlist_id).await
    }

    pub async fn unsave_cus_playlist(&self, playlist_id: &str) -> YResult<()> {
        self.dao.unsave_cus_playlist_raw(playlist_id).await
    }
}
