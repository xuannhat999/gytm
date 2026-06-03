use data::{PlayList, Song};
use error::{YError, YResult, log_to_file};
use reqwest::{
    Client, Url,
    cookie::Jar,
    header::{HeaderMap, HeaderValue},
};
use rookie::{any_browser, common::enums::Cookie, load};
use serde_json::{Value, json};
use std::{path::PathBuf, sync::Arc};
pub mod parser;

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

    pub async fn get_raw_lists(&self) -> YResult<Value> {
        let url = format!(
            "{}/youtubei/v1/browse?key={}&alt=json",
            YTM_DOMAIN, self.innertube_api_key
        );

        let body = json!({
            "context": {
                "client": {
                    "clientName": "WEB_REMIX",
                    "clientVersion": self.client_version,
                }
            },
            "browseId": "FEmusic_library_landing",
        });

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

    pub async fn get_continuation_data(&self, token: &str) -> YResult<Value> {
        let url = format!(
            "{}/youtubei/v1/browse?key={}&alt=json",
            YTM_DOMAIN, self.innertube_api_key
        );

        let body = json!({
            "context": {
                "client": {
                    "clientName": "WEB_REMIX",
                    "clientVersion": self.client_version,
                }
            },
            "continuation": token,
        });

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
    pub async fn get_raw_songs(&self, id: &str) -> YResult<Value> {
        let url = format!(
            "{}/youtubei/v1/browse?key={}&alt=json",
            YTM_DOMAIN, self.innertube_api_key
        );
        let body = json!({
            "context": {
                "client": {
                    "clientName": "WEB_REMIX",
                    "clientVersion": self.client_version,
                }
            },
            "browseId": id.to_string(),
        });
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

        let body = json!({
            "context": {
                "client": {
                    "clientName": "WEB_REMIX",
                    "clientVersion": self.client_version,
                }
            },
            "query": query,
            "params": params,
        });
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

    pub async fn get_raw_params(&self, video_id: &str) -> YResult<Value> {
        let url = format!(
            "{}/youtubei/v1/next?key={}&alt=json",
            YTM_DOMAIN, self.innertube_api_key
        );
        let body = json!({
            "context": {
                "client": {
                    "clientName": "WEB_REMIX",
                    "clientVersion": self.client_version,
                }
            },
            "videoId": video_id
        });
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

    pub async fn get_raw_related_songs(&self, playlist_id: &str, params: &str) -> YResult<Value> {
        let url = format!(
            "{}/youtubei/v1/next?key={}&alt=json",
            YTM_DOMAIN, self.innertube_api_key
        );

        let body = json!({
            "context": {
                "client": {
                    "clientName": "WEB_REMIX",
                    "clientVersion": self.client_version,
                }
            },
            "playlistId": playlist_id,
            "params": params,
            "tunerSettingValue": "AUTOMIX_SETTING_NORMAL"
        });

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
    pub async fn add_to_lib(&self, playlist_id: &str) -> YResult<Value> {
        let url = format!(
            "{}/youtubei/v1/like/like?key={}&alt=json",
            YTM_DOMAIN, self.innertube_api_key
        );
        let body = json!({
            "context": {
                "client": {
                    "clientName": "WEB_REMIX",
                    "clientVersion": self.client_version,
                }
            },
            "target": {
                "playlistId": playlist_id
            },
            "status": "LIKE"
        });

        let response = self
            .http
            .post(&url)
            .headers(self.get_api_headers()?)
            .json(&body)
            .send()
            .await?;

        let text = response.text().await?;

        if text.trim().is_empty() {
            Ok(json!({}))
        } else {
            let json_val: Value = serde_json::from_str(&text)?;
            Ok(json_val)
        }
    }

    // REMOVE SAVED ALBUM IN LIBRARY
    pub async fn remove_from_lib(&self, playlist_id: &str) -> YResult<Value> {
        let url = format!(
            "{}/youtubei/v1/like/removelike?key={}&alt=json",
            YTM_DOMAIN, self.innertube_api_key
        );
        let body = json!({
            "context": {
                "client": {
                    "clientName": "WEB_REMIX",
                    "clientVersion": self.client_version,
                }
            },
            "target": {
                "playlistId": playlist_id
            },
        });

        let response = self
            .http
            .post(&url)
            .headers(self.get_api_headers()?)
            .json(&body)
            .send()
            .await?;

        let text = response.text().await?;

        if text.trim().is_empty() {
            Ok(json!({}))
        } else {
            let json_val: Value = serde_json::from_str(&text)?;
            Ok(json_val)
        }
    }

    // FETCH ALBUMS/PLAYLIST IN LIBRARY
    pub async fn get_lists(&self) -> YResult<(Vec<PlayList>, Vec<PlayList>)> {
        let mut all_albums: Vec<PlayList> = Vec::new();
        let mut all_playlists: Vec<PlayList> = Vec::new();
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
            let next_raw_data = self.get_continuation_data(&current_token).await?;
            let (mut next_albums, mut next_playlists, next_token) =
                parser::parse_lists(next_raw_data)?;
            all_albums.append(&mut next_albums);
            all_playlists.append(&mut next_playlists);
            token = next_token;
        }
        Ok((all_albums, all_playlists))
    }

    // FETCH SONGS FROM ALBUM/PLAYLIST
    pub async fn get_songs(&self, browse_id: &str) -> YResult<Vec<Song>> {
        let raw_songs = match self.get_raw_songs(browse_id).await {
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
    pub async fn get_search_albums(&self, query: &str) -> YResult<Vec<PlayList>> {
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
        let raw_data = match self.get_raw_params(video_id).await {
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
        let raw_data = match self.get_raw_related_songs(&playlist_id, params).await {
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
    let browser_dirs = vec!["mozilla/firefox", "librewolf/librewolf", "zen"];

    let config_dir = dirs::config_dir().ok_or_else(|| YError::ConfigDirError)?;
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
                    let db_path = path.join("cookies.sqlite");
                    if db_path.exists() {
                        target_db_path = Some(db_path);
                        break 'outer;
                    }
                }
            }
        }
    }

    let cookies_path = target_db_path
        .ok_or_else(|| YError::InvalidFilePath)?
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
