use crate::{client::load_cookies, dao::YTDao, parser};
use data::{
    api_client::{Account, SearchType},
    app::{PlayListPrivacy, Playlist, Song},
};
use error::{
    YError::{self},
    YResult, log_to_file,
};
use state::client_state::ClientState;

pub struct YTBus {
    dao: YTDao,
    pending_dao: Option<YTDao>,
}

impl YTBus {
    pub fn new(dao: YTDao) -> Self {
        Self {
            dao,
            pending_dao: None,
        }
    }
    pub async fn reload_client(&mut self, client_state: &ClientState) -> YResult<()> {
        self.pending_dao = None;
        let (browser, profile, container, account) = &client_state.get_validated_fields()?;
        let (jar, sapisid) = load_cookies(browser, profile, *container)?;
        self.dao
            .reload(jar, sapisid, account.map_or(0, |a| a.auth_user))
            .await
    }

    pub async fn toggle_guest(&mut self) -> YResult<()> {
        self.pending_dao = None;
        self.dao = YTDao::default().await?;
        Ok(())
    }

    pub async fn set_client(&mut self, client_state: &ClientState) -> YResult<()> {
        let (_, _, _, account) = client_state.get_validated_fields()?;
        match self.pending_dao.take() {
            Some(mut dao) => {
                if dao.sapisid.is_none() {
                    return Err(YError::InvalidCookie);
                }
                dao.auth_user = account.map_or(0, |a| a.auth_user);
                self.dao = dao;
                Ok(())
            }
            None => self.reload_client(client_state).await,
        }
    }

    pub fn discard_pending_client(&mut self) {
        self.pending_dao = None;
    }

    pub async fn create_playlist(
        &self,
        title: &str,
        desc: &str,
        privacy: PlayListPrivacy,
    ) -> YResult<Playlist> {
        self.check_auth()?;
        let res = self.dao.create_playlist(title, desc, privacy).await?;
        let playlist = parser::parse_created_playlist(&res)?;
        Ok(playlist)
    }

    pub async fn get_lists(&self) -> YResult<(Vec<Playlist>, Vec<Playlist>, Vec<usize>)> {
        let mut all_albums: Vec<Playlist> = Vec::new();
        let mut all_playlists: Vec<Playlist> = Vec::new();
        let mut all_cus_playlists: Vec<usize> = Vec::new();
        let raw_data = self.dao.get_library_playlists().await?;

        let (mut albums, mut playlists, mut token) = parser::parse_lists(&raw_data)?;
        all_albums.append(&mut albums);
        all_playlists.append(&mut playlists);
        while let Some(current_token) = token {
            let next_raw_data = self.dao.get_continuation(&current_token).await?;
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

    pub async fn get_accounts_list(&mut self, client_state: &ClientState) -> YResult<Vec<Account>> {
        let dao = YTDao::new(client_state).await?;
        if dao.sapisid.is_none() {
            return Err(YError::InvalidCookie);
        }
        let mut emails = Vec::new();
        let mut auth_user = 0;
        while let Ok(res) = dao.get_account_email_from_idx(auth_user).await {
            match parser::parse_account(&res) {
                Ok(email) => {
                    emails.push(Account { email, auth_user });
                    auth_user += 1;
                }
                Err(_) => {
                    break;
                }
            }
        }
        self.pending_dao = Some(dao);
        Ok(emails)
    }

    pub async fn get_songs(&self, browse_id: &str) -> YResult<Vec<Song>> {
        let raw = self.dao.get_songs_from_browse_id(browse_id).await?;
        parser::parse_songs(&raw)
    }

    // SEARCH ALBUMS
    pub async fn get_search_albums(&self, query: &str) -> YResult<Vec<Playlist>> {
        let raw_list = self
            .dao
            .search_with_params(query, SearchType::Album)
            .await?;
        parser::parse_search_albums(&raw_list)
    }

    // SEARCH SONGS
    pub async fn get_search_songs(&self, query: &str) -> YResult<Vec<Song>> {
        let raw_songs = self.dao.search_with_params(query, SearchType::Song).await?;
        parser::parse_search_songs(&raw_songs)
    }

    // SEARCH VIDEOS
    pub async fn get_search_videos(&self, query: &str) -> YResult<Vec<Song>> {
        let videos_raw = self
            .dao
            .search_with_params(query, SearchType::Video)
            .await?;
        parser::parse_search_songs(&videos_raw)
    }

    pub async fn get_params(&self, video_id: &str) -> YResult<String> {
        let raw_data = self.dao.get_params(video_id).await?;
        parser::parse_params(&raw_data)
    }

    pub async fn get_related_songs(&self, song: Song, params: &str) -> YResult<Vec<Song>> {
        let video_id = &song.video_id;
        let playlist_id = format!("RDAMVM{}", video_id);
        let raw_data = self.dao.get_related_songs(&playlist_id, params).await?;
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
        self.check_auth()?;
        self.dao.save_to_playlist(song, playlist_id).await
    }

    pub async fn unsave_to_playlist(&self, song: &Song, playlist_id: &str) -> YResult<()> {
        self.check_auth()?;
        self.dao.unsave_to_playlist(song, playlist_id).await
    }

    pub async fn like_song(&self, song: &Song) -> YResult<()> {
        self.check_auth()?;
        self.dao.like_song(song).await
    }

    pub async fn unlike_song(&self, song: &Song) -> YResult<()> {
        self.check_auth()?;
        self.dao.unlike_song(song).await
    }

    pub async fn save_album(&self, playlist_id: &str) -> YResult<()> {
        self.check_auth()?;
        self.dao.save_album(playlist_id).await
    }

    pub async fn unsave_album(&self, playlist_id: &str) -> YResult<()> {
        self.check_auth()?;
        self.dao.unsave_album(playlist_id).await
    }

    pub async fn unsave_cus_playlist(&self, playlist_id: &str) -> YResult<()> {
        self.check_auth()?;
        self.dao.unsave_cus_playlist(playlist_id).await
    }

    fn check_auth(&self) -> YResult<()> {
        if self.dao.sapisid.is_none() {
            return Err(YError::UnavailableFeature);
        }
        Ok(())
    }
}
