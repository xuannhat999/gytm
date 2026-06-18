use data::{PlayListPrivacy, Playlist, Song};
use error::{YError, YResult, log_to_file};
use reqwest::{
    Client, Url,
    cookie::Jar,
    header::{HeaderMap, HeaderValue},
};
use rookie::{any_browser, common::enums::Cookie, load};
use serde_json::Value;
use std::{path::PathBuf, sync::Arc};

use crate::request::{
    ActionsContent, BrowseIdRequest, CreatePlaylistRequest, GetContinuationRequest,
    GetRelatedSongsRequest, PlaylistIdRequest, RequestClient, RequestContext, SaveAlbumRequest,
    SaveUnsaveListRequest, SearchRequest, TargetContent, TargetRequest, VideoIdRequest,
};
pub mod parser;
pub mod protocol;
pub mod request;
pub struct YClient {
    pub http: Client,
    pub sapisid: String,
    pub innertube_api_key: String,
    pub client_version: String,
}

const YTM_DOMAIN: &str = "https://music.youtube.com";

impl YClient {
    pub async fn new() -> YResult<Self> {
        let (jar, sapisid) = load_cookies()?;
        let http = Client::builder()
            .cookie_provider(Arc::new(jar))
            .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .build()?;

        let response_text = http.get(YTM_DOMAIN).send().await?.text().await?;

        let innertube_api_key = extract_between(&response_text, "INNERTUBE_API_KEY\":\"", "\"")
            .ok_or_else(|| YError::InvalidCookie)?;

        let client_version = extract_between(&response_text, "INNERTUBE_CLIENT_VERSION\":\"", "\"")
            .ok_or_else(|| YError::InvalidCookie)?;

        Ok(Self {
            http,
            sapisid,
            innertube_api_key,
            client_version,
        })
    }

    // This function is adapted from: https://github.com/ccgauche/ytermusic.git
    // Original source: https://github.com/ccgauche/ytermusic/blob/master/crates/ytpapi2/src/lib.rs
    fn compute_sapi_hash(&self) -> String {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut hasher = sha1_smol::Sha1::new();

        let message = format!("{timestamp} {} {YTM_DOMAIN}", self.sapisid);
        hasher.update(message.as_bytes());

        let result = hasher.digest();

        let hex_hash = result.to_string();

        format!("{}_{}", timestamp, hex_hash)
    }

    // This function is adapted from: https://github.com/ccgauche/ytermusic.git
    // Original source: https://github.com/ccgauche/ytermusic/blob/master/crates/ytpapi2/src/lib.rs
    pub fn get_api_headers(&self) -> YResult<HeaderMap> {
        let mut headers = HeaderMap::new();
        headers.insert("Content-Type", HeaderValue::from_static("application/json"));
        headers.insert("Origin", HeaderValue::from_static(YTM_DOMAIN));
        headers.insert("X-Goog-AuthUser", HeaderValue::from_static("0"));
        let auth_val = HeaderValue::from_str(&format!("SAPISIDHASH {}", self.compute_sapi_hash()))
            .map_err(YError::InvalidHeader);
        headers.insert("Authorization", auth_val?);
        Ok(headers)
    }
    fn get_context(&self) -> RequestContext<'_> {
        RequestContext {
            client: RequestClient {
                client_name: "WEB_REMIX",
                client_version: &self.client_version,
            },
        }
    }
    pub async fn get_raw_lists(&self) -> YResult<Value> {
        let url = format!(
            "{}/youtubei/v1/browse?key={}&alt=json",
            YTM_DOMAIN, self.innertube_api_key
        );

        let body = BrowseIdRequest {
            context: self.get_context(),
            browse_id: "FEmusic_library_landing",
        };
        let response = self
            .http
            .post(&url)
            .headers(self.get_api_headers()?)
            .json(&body)
            .send()
            .await?
            .json::<Value>()
            .await?;
        Ok(response)
    }

    pub async fn get_continuation_raw(&self, token: &str) -> YResult<Value> {
        let url = format!(
            "{}/youtubei/v1/browse?key={}&alt=json",
            YTM_DOMAIN, self.innertube_api_key
        );

        let body = GetContinuationRequest {
            context: self.get_context(),
            continuation: token,
        };
        let response = self
            .http
            .post(&url)
            .headers(self.get_api_headers()?)
            .json(&body)
            .send()
            .await?
            .json::<Value>()
            .await?;

        Ok(response)
    }
    pub async fn get_songs_raw(&self, browse_id: &str) -> YResult<Value> {
        let url = format!(
            "{}/youtubei/v1/browse?key={}&alt=json",
            YTM_DOMAIN, self.innertube_api_key
        );
        let body = BrowseIdRequest {
            context: self.get_context(),
            browse_id,
        };
        let response = self
            .http
            .post(&url)
            .headers(self.get_api_headers()?)
            .json(&body)
            .send()
            .await?
            .json::<Value>()
            .await?;
        Ok(response)
    }

    pub async fn get_raw_search_albums(&self, query: &str, rtype: u8) -> YResult<Value> {
        let params = if rtype == 1 {
            "EgWKAQIIAWoSEAQQAxAFEAoQDhAJEBUQEBAR" // SONG
        } else {
            "EgWKAQIYAWoSEAUQAxAJEAQQChAQEBUQDhAR" // ALBUM
        };
        let url = format!(
            "{}/youtubei/v1/search?key={}&alt=json",
            YTM_DOMAIN, self.innertube_api_key
        );

        let body = SearchRequest {
            context: self.get_context(),
            query,
            params,
        };
        let response = self
            .http
            .post(&url)
            .headers(self.get_api_headers()?)
            .json(&body)
            .send()
            .await?
            .json::<Value>()
            .await?;

        Ok(response)
    }

    pub async fn get_params_raw(&self, video_id: &str) -> YResult<Value> {
        let url = format!(
            "{}/youtubei/v1/next?key={}&alt=json",
            YTM_DOMAIN, self.innertube_api_key
        );
        let body = VideoIdRequest {
            context: self.get_context(),
            video_id,
        };
        let response = self
            .http
            .post(&url)
            .headers(self.get_api_headers()?)
            .json(&body)
            .send()
            .await?
            .json::<Value>()
            .await?;
        Ok(response)
    }

    pub async fn create_playlist_raw(
        &self,
        title: &str,
        desc: &str,
        privacy: PlayListPrivacy,
    ) -> YResult<Value> {
        let url = format!(
            "{}/youtubei/v1/playlist/create?key={}&alt=json",
            YTM_DOMAIN, self.innertube_api_key
        );
        let body = CreatePlaylistRequest {
            context: self.get_context(),
            title,
            params: "KAA%3D",
            description: if desc.is_empty() { None } else { Some(desc) },
            privacy_status: if privacy == PlayListPrivacy::Private {
                None
            } else {
                Some(privacy)
            },
        };
        let response = self
            .http
            .post(&url)
            .headers(self.get_api_headers()?)
            .json(&body)
            .send()
            .await?
            .json::<Value>()
            .await?;
        Ok(response)
    }

    pub async fn create_playlist(
        &self,
        title: &str,
        desc: &str,
        privacy: PlayListPrivacy,
    ) -> YResult<Playlist> {
        let res = self.create_playlist_raw(title, desc, privacy).await?;
        let playlist = parser::parse_created_playlist(res)?;
        Ok(playlist)
    }

    pub async fn get_related_songs_raw(&self, playlist_id: &str, params: &str) -> YResult<Value> {
        let url = format!(
            "{}/youtubei/v1/next?key={}&alt=json",
            YTM_DOMAIN, self.innertube_api_key
        );
        let body = GetRelatedSongsRequest {
            context: self.get_context(),
            playlist_id,
            params,
            tuner_setting_value: "AUTOMIX_SETTING_NORMAL",
        };
        let response = self
            .http
            .post(&url)
            .headers(self.get_api_headers()?)
            .json(&body)
            .send()
            .await?
            .json::<Value>()
            .await?;

        Ok(response)
    }

    // SAVE ALBUM TO LIBRARY
    pub async fn save_album_raw(&self, playlist_id: &str) -> YResult<Value> {
        let url = format!(
            "{}/youtubei/v1/like/like?key={}&alt=json",
            YTM_DOMAIN, self.innertube_api_key
        );
        let body = SaveAlbumRequest {
            context: self.get_context(),
            target: TargetContent {
                playlist_id: Some(playlist_id),
                video_id: None,
            },
            status: "LIKE",
        };

        let response = self
            .http
            .post(&url)
            .headers(self.get_api_headers()?)
            .json(&body)
            .send()
            .await?
            .json::<Value>()
            .await?;
        Ok(response)
    }

    pub async fn unsave_cus_playlist_raw(&self, playlist_id: &str) -> YResult<Value> {
        let url = format!(
            "{}/youtubei/v1/playlist/delete?key={}&alt=json",
            YTM_DOMAIN, self.innertube_api_key
        );

        let body = PlaylistIdRequest {
            context: self.get_context(),
            playlist_id,
        };
        let response = self
            .http
            .post(&url)
            .headers(self.get_api_headers()?)
            .json(&body)
            .send()
            .await?
            .json::<Value>()
            .await?;
        Ok(response)
    }

    // REMOVE SAVED ALBUM IN LIBRARY
    pub async fn unsave_album_raw(&self, playlist_id: &str) -> YResult<Value> {
        let url = format!(
            "{}/youtubei/v1/like/removelike?key={}&alt=json",
            YTM_DOMAIN, self.innertube_api_key
        );
        let body = TargetRequest {
            context: self.get_context(),
            target: TargetContent {
                playlist_id: Some(playlist_id),
                video_id: None,
            },
        };

        let response = self
            .http
            .post(&url)
            .headers(self.get_api_headers()?)
            .json(&body)
            .send()
            .await?
            .json::<Value>()
            .await?;
        Ok(response)
    }

    pub async fn save_to_playlist(&self, song: &Song, playlist_id: &str) -> YResult<()> {
        let url = format!(
            "{}/youtubei/v1/browse/edit_playlist?key={}&alt=json",
            YTM_DOMAIN, self.innertube_api_key
        );
        let video_id = &song.video_id;
        let actions = vec![ActionsContent {
            action: "ACTION_ADD_VIDEO",
            added_video_id: Some(video_id),
            dedupe_option: Some("DEDUPE_OPTION_CHECK"),
            removed_video_id: None,
            set_video_id: None,
        }];

        let body = SaveUnsaveListRequest {
            context: self.get_context(),
            actions,
            playlist_id,
        };
        let response = self
            .http
            .post(&url)
            .headers(self.get_api_headers()?)
            .json(&body)
            .send()
            .await?;

        let status = response.status();
        let bytes = response.bytes().await?;

        let val: Value = serde_json::from_slice(&bytes)?;

        match val["status"].as_str() {
            Some("STATUS_SUCCEEDED") => Ok(()),
            _ if status.is_success() => Err(YError::AlreadyInPlaylist),
            _ => Err(YError::BadStatus(String::from(
                "Save/Unsave song to playlist",
            ))),
        }
    }

    // REMOVE SONG FROM PLAYLIST
    pub async fn unsave_to_playlist(&self, song: &Song, playlist_id: &str) -> YResult<()> {
        let video_id = &song.video_id;
        let set_video_id = &song.set_video_id;
        let url = format!(
            "{}/youtubei/v1/browse/edit_playlist?key={}&alt=json",
            YTM_DOMAIN, self.innertube_api_key
        );
        let actions = vec![ActionsContent {
            action: "ACTION_REMOVE_VIDEO",
            added_video_id: None,
            dedupe_option: None,
            removed_video_id: Some(video_id),
            set_video_id: Some(set_video_id),
        }];

        let body = SaveUnsaveListRequest {
            context: self.get_context(),
            actions,
            playlist_id,
        };
        let response = self
            .http
            .post(&url)
            .headers(self.get_api_headers()?)
            .json(&body)
            .send()
            .await?;

        let status = response.status();
        let bytes = response.bytes().await?;

        let val: Value = serde_json::from_slice(&bytes)?;

        match val["status"].as_str() {
            Some("STATUS_SUCCEEDED") => Ok(()),
            _ if status.is_success() => Err(YError::AlreadyInPlaylist),
            _ => Err(YError::BadStatus(String::from(
                "Save/Unsave song to playlist",
            ))),
        }
    }
    pub async fn unlike_song(&self, song: &Song) -> YResult<()> {
        let video_id = &song.video_id;
        let url = format!(
            "{}/youtubei/v1/like/removelike?key={}&alt=json",
            YTM_DOMAIN, self.innertube_api_key
        );
        let body = TargetRequest {
            context: self.get_context(),
            target: TargetContent {
                video_id: Some(video_id),
                playlist_id: None,
            },
        };
        let response = self
            .http
            .post(&url)
            .headers(self.get_api_headers()?)
            .json(&body)
            .send()
            .await?;

        let status = response.status();
        if status.is_success() {
            Ok(())
        } else {
            Err(YError::BadStatus(String::from("Like Song")))
        }
    }

    pub async fn like_song(&self, song: &Song) -> YResult<()> {
        let video_id = &song.video_id;
        let url = format!(
            "{}/youtubei/v1/like/like?key={}&alt=json",
            YTM_DOMAIN, self.innertube_api_key
        );

        let body = TargetRequest {
            context: self.get_context(),
            target: TargetContent {
                video_id: Some(video_id),
                playlist_id: None,
            },
        };
        let response = self
            .http
            .post(&url)
            .headers(self.get_api_headers()?)
            .json(&body)
            .send()
            .await?;

        let status = response.status();
        if status.is_success() {
            Ok(())
        } else {
            Err(YError::BadStatus(String::from("Like Song")))
        }
    }

    // FETCH ALBUMS/PLAYLIST IN LIBRARY
    pub async fn get_lists(&self) -> YResult<(Vec<Playlist>, Vec<Playlist>, Vec<usize>)> {
        let mut all_albums: Vec<Playlist> = Vec::new();
        let mut all_playlists: Vec<Playlist> = Vec::new();
        let mut all_cus_playlists: Vec<usize> = Vec::new();
        let raw_data = match self.get_raw_lists().await {
            Ok(raw_lists) => raw_lists,
            Err(e) => {
                log_to_file(&e);
                Value::Null
            }
        };

        let (mut albums, mut playlists, mut token) = match parser::parse_lists(raw_data) {
            Ok((albums, playlists, token)) => (albums, playlists, token),
            Err(e) => {
                log_to_file(&e);
                (Vec::new(), Vec::new(), None)
            }
        };
        all_albums.append(&mut albums);
        all_playlists.append(&mut playlists);

        while let Some(current_token) = token {
            let next_raw_data = self.get_continuation_raw(&current_token).await?;
            let (mut next_albums, mut next_playlists, next_token) =
                parser::parse_lists(next_raw_data)?;
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

    // FETCH SONGS FROM ALBUM/PLAYLIST
    pub async fn get_songs(&self, browse_id: &str) -> YResult<Vec<Song>> {
        let raw_songs = match self.get_songs_raw(browse_id).await {
            Ok(raw_songs) => raw_songs,
            Err(e) => {
                log_to_file(&e);
                Value::Null
            }
        };
        let songs = match parser::parse_songs(raw_songs) {
            Ok(songs) => songs,
            Err(e) => {
                log_to_file(&e);
                Vec::new()
            }
        };
        Ok(songs)
    }

    // FETCH SEARCH RESULT ALBUMS
    pub async fn get_search_albums(&self, query: &str) -> YResult<Vec<Playlist>> {
        let raw_list = match self.get_raw_search_albums(query, 2).await {
            Ok(raw_data) => raw_data,
            Err(e) => {
                log_to_file(&e);
                Value::Null
            }
        };
        let albums = match parser::parse_search_albums(raw_list) {
            Ok(list) => list,
            Err(e) => {
                log_to_file(&e);
                Vec::new()
            }
        };
        Ok(albums)
    }

    // FETCH SEARCH REUSLT SONGS
    pub async fn get_search_songs(&self, query: &str) -> YResult<Vec<Song>> {
        let raw_data = match self.get_raw_search_albums(query, 1).await {
            Ok(raw) => raw,
            Err(e) => {
                log_to_file(&e);
                Value::Null
            }
        };
        let songs = match parser::parse_search_songs(raw_data) {
            Ok(songs) => songs,
            Err(e) => {
                log_to_file(&e);
                Vec::new()
            }
        };
        Ok(songs)
    }

    // FETCH PARAMS
    pub async fn get_params(&self, video_id: &str) -> YResult<String> {
        let raw_data = match self.get_params_raw(video_id).await {
            Ok(raw) => raw,
            Err(e) => {
                log_to_file(&e);
                Value::Null
            }
        };
        let params = match parser::parse_params(raw_data) {
            Ok(params) => params,
            Err(e) => {
                log_to_file(&e);
                String::new()
            }
        };
        Ok(params)
    }

    // FETCH RELATED SONGS
    pub async fn get_related_songs(&self, video_id: &str, params: &str) -> YResult<Vec<Song>> {
        let playlist_id = format!("RDAMVM{}", video_id);
        let raw_data = match self.get_related_songs_raw(&playlist_id, params).await {
            Ok(raw) => raw,
            Err(e) => {
                log_to_file(&e);
                Value::Null
            }
        };
        let songs = match parser::parse_related_songs(raw_data) {
            Ok(songs) => songs,
            Err(e) => {
                log_to_file(&e);
                Vec::new()
            }
        };
        Ok(songs)
    }
}

// ONLY WORKS WITH CHROMIUM BASED BROWSER ( No idea )
pub fn load_cookies() -> YResult<(Jar, String)> {
    let jar = Jar::default();
    let url = YTM_DOMAIN.parse::<Url>()?;
    let domains = vec!["youtube.com".to_string(), "music.youtube.com".to_string()];
    let mut sapisid_extracted = String::new();
    let mut cookies = load(Some(domains)).unwrap_or_else(|_| Vec::new());
    if cookies.is_empty() {
        cookies = load_cookies_firefox_based()?;
    }

    for cookie in cookies {
        if cookie.name == "SAPISID" {
            sapisid_extracted = cookie.value.clone();
        }
        let cookie_str = format!(
            "{}={}; Path={}; Secure; HttpOnly",
            cookie.name, cookie.value, cookie.path
        );
        jar.add_cookie_str(&cookie_str, &url);
    }
    if sapisid_extracted.is_empty() {
        return Err(YError::InvalidCookie);
    }

    Ok((jar, sapisid_extracted))
}

pub fn load_cookies_firefox_based() -> YResult<Vec<Cookie>> {
    let domains = vec!["youtube.com".to_string(), "music.youtube.com".to_string()];
    let browser_dirs = vec![
        "mozilla/firefox",
        "librewolf/librewolf",
        "zen",
        "BraveSoftware/Brave-Origin",
    ];
    let target_filename = vec!["cookies.sqlite", "Cookies"];
    let config_dir =
        dirs::config_dir().ok_or_else(|| YError::InvalidPath("~/.config/".to_string()))?;
    let mut target_db_path: Option<PathBuf> = None;
    'outer: for browser in browser_dirs {
        let base_path = config_dir.join(browser);
        if !base_path.exists() {
            continue;
        }
        if let Ok(entries) = std::fs::read_dir(base_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    for file_name in &target_filename {
                        let db_path = path.join(file_name);
                        if db_path.exists() {
                            target_db_path = Some(db_path);
                            break 'outer;
                        }
                    }
                }
            }
        }
    }

    let cookies_path = target_db_path
        .ok_or_else(|| YError::InvalidPath("~/.config/browser_dirs/../Cookie".to_string()))?
        .to_string_lossy()
        .into_owned();

    let cookies =
        any_browser(&cookies_path, Some(domains), None).map_err(|_| YError::InvalidCookie)?;

    Ok(cookies)
}

fn extract_between(source: &str, start: &str, end: &str) -> Option<String> {
    source.find(start).and_then(|start_idx| {
        let start_pos = start_idx + start.len();
        source[start_pos..]
            .find(end)
            .map(|end_idx| source[start_pos..start_pos + end_idx].to_string())
    })
}
