use data::AppConfig;
use error::{Result, YError};
use reqwest::{
    Client, Url,
    cookie::Jar,
    header::{HeaderMap, HeaderValue},
};
use rookie::load;
use serde_json::{Value, json};
use sha1::Digest;
use std::sync::Arc;

pub struct YClient {
    pub http: Client,
    pub sapisid: String,
    pub innertube_api_key: String,
    pub client_version: String,
    pub app_config: AppConfig,
}

const YTM_DOMAIN: &str = "https://music.youtube.com";

impl YClient {
    pub async fn new(config: AppConfig) -> Result<Self> {
        let (jar, sapisid) = load_auto_cookies()?;

        let http = Client::builder()
            .cookie_provider(Arc::new(jar))
            .user_agent(&config.user_agent)
            .build()?;

        let response_text = http.get(YTM_DOMAIN).send().await?.text().await?;

        let api_key = extract_between(&response_text, "INNERTUBE_API_KEY\":\"", "\"")
            .ok_or_else(|| YError::InvalidCookie)?;

        let client_version = extract_between(&response_text, "INNERTUBE_CLIENT_VERSION\":\"", "\"")
            .ok_or_else(|| YError::InvalidCookie)?;

        Ok(Self {
            http,
            sapisid,
            innertube_api_key: api_key.to_string(),
            client_version: client_version.to_string(),
            app_config: config,
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

    pub async fn get_lib_data(&self) -> Result<Value> {
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

    pub async fn get_playlist_songs(&self, id: &str) -> Result<Value> {
        let url = format!(
            "{}/youtubei/v1/browse?key={}&alt=json",
            YTM_DOMAIN, self.innertube_api_key
        );
        let body = json!({
            "context": {
                "client": {
                    "clientName": "WEB_REMIX",
                    "clientVersion": self.client_version,
                    "hl": "vi",
                    "gl": "VN"
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
}

pub fn load_auto_cookies() -> Result<(Jar, String)> {
    let jar = Jar::default();
    let url = "https://music.youtube.com".parse::<Url>()?;

    let domains = vec!["youtube.com".to_string(), "music.youtube.com".to_string()];

    let cookies = load(Some(domains)).map_err(|e| YError::CookieError(e.to_string()))?;
    let mut sapisid_extracted = String::new();

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

