use data::{PlayList, Song};
use error::{Result, YError, log_to_file};
use reqwest::{
    Client, Url,
    cookie::Jar,
    header::{HeaderMap, HeaderValue},
};
use rookie::{any_browser, common::enums::Cookie, load};
use serde_json::{Value, json};
use sha1::Digest;
use state::AppState;
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
    pub async fn new(config: &AppState) -> Result<Self> {
        let (jar, sapisid) = load_cookies()?;

        let http = Client::builder()
            .cookie_provider(Arc::new(jar))
            .user_agent(&config.user_agent)
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

        let mut hasher = sha1::Sha1::new();
        hasher.update(format!("{timestamp} {} {YTM_DOMAIN}", self.sapisid));
        let result = hasher.finalize();

        let hex_hash = result
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>();
        format!("{}_{}", timestamp, hex_hash)
    }

    // This function is adapted from: https://github.com/ccgauche/ytermusic.git
    // Original source: https://github.com/ccgauche/ytermusic/blob/master/crates/ytpapi2/src/lib.rs
    pub fn get_api_headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert("Content-Type", HeaderValue::from_static("application/json"));
        headers.insert("Origin", HeaderValue::from_static(YTM_DOMAIN));
        headers.insert("X-Goog-AuthUser", HeaderValue::from_static("0"));
        headers.insert(
            "Authorization",
            HeaderValue::from_str(&format!("SAPISIDHASH {}", self.compute_sapi_hash())).unwrap(),
        );
        headers
    }

    pub async fn get_raw_lists(&self) -> Result<Value> {
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
            .headers(self.get_api_headers())
            .json(&body)
            .send()
            .await?
            .json::<Value>()
            .await?;
        Ok(response)
    }

    pub async fn get_continuation_data(&self, token: &str) -> Result<Value> {
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
            .headers(self.get_api_headers())
            .json(&body)
            .send()
            .await?
            .json::<Value>()
            .await?;

        Ok(response)
    }
    pub async fn get_raw_songs(&self, id: &str) -> Result<Value> {
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
            .headers(self.get_api_headers())
            .json(&body)
            .send()
            .await?
            .json::<Value>()
            .await?;
        Ok(response)
    }

    pub async fn get_lists(&self) -> Result<(Vec<PlayList>, Vec<PlayList>)> {
        let mut all_albums: Vec<PlayList> = Vec::new();
        let mut all_playlists: Vec<PlayList> = Vec::new();
        let raw_data = match self.get_raw_lists().await {
            Ok(raw_lists) => raw_lists,
            Err(e) => {
                log_to_file(&e);
                Value::Null
            }
        };

        let (mut albums, mut playlists, mut token) = match parser::extract_lists(raw_data) {
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
                parser::extract_lists(next_raw_data)?;
            all_albums.append(&mut next_albums);
            all_playlists.append(&mut next_playlists);
            token = next_token;
        }
        Ok((all_albums, all_playlists))
    }

    pub async fn get_songs(&self, id: &str) -> Result<Vec<Song>> {
        let raw_songs = match self.get_raw_songs(id).await {
            Ok(raw_songs) => raw_songs,
            Err(e) => {
                log_to_file(&e);
                Value::Null
            }
        };
        let songs = match parser::extract_songs(raw_songs) {
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
pub fn load_cookies() -> Result<(Jar, String)> {
    let jar = Jar::default();
    let url = YTM_DOMAIN.parse::<Url>()?;

    let domains = vec!["youtube.com".to_string(), "music.youtube.com".to_string()];

    let mut cookies = load(Some(domains)).map_err(|_| YError::InvalidCookie)?;
    let mut sapisid_extracted = String::new();
    if cookies.is_empty() {
        cookies = load_cookies_firefox_based()?;
    }

    for cookie in cookies {
        if cookie.name == "SAPISID" {
            sapisid_extracted = cookie.value.clone();
        }
        let cookie_str = format!("{}={}", cookie.name, cookie.value);
        jar.add_cookie_str(&cookie_str, &url);
    }

    if sapisid_extracted.is_empty() {
        return Err(YError::InvalidCookie);
    }

    Ok((jar, sapisid_extracted))
}

fn extract_between(source: &str, start: &str, end: &str) -> Option<String> {
    source.find(start).and_then(|start_idx| {
        let start_pos = start_idx + start.len();
        source[start_pos..]
            .find(end)
            .map(|end_idx| source[start_pos..start_pos + end_idx].to_string())
    })
}

pub fn load_cookies_firefox_based() -> Result<Vec<Cookie>> {
    let domains = vec!["youtube.com".to_string(), "music.youtube.com".to_string()];
    let browser_dirs = vec!["mozilla/firefox", "librewolf/librewolf", "zen"];

    let config_dir = dirs::config_dir().ok_or_else(|| YError::InvalidCookie)?;
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

    let db_path_buf = target_db_path.ok_or_else(|| YError::InvalidCookie)?;
    let cookies_path = db_path_buf.to_str().ok_or_else(|| YError::InvalidCookie)?;

    let cookies =
        any_browser(cookies_path, Some(domains), None).map_err(|_| YError::InvalidCookie)?;

    Ok(cookies)
}
