use data::AppConfig;
use error::{Result, YError};
use reqwest::{
    Client,
    header::{COOKIE, HeaderMap, HeaderValue, USER_AGENT},
};
use serde_json::{Value, json};
use sha1::Digest;

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
        let mut headers = HeaderMap::new();
        headers.insert(COOKIE, HeaderValue::from_str(&config.cookie)?);
        headers.insert(USER_AGENT, HeaderValue::from_str(&config.user_agent)?);

        let http = Client::builder().default_headers(headers.clone()).build()?;

        let response_text = http.get(YTM_DOMAIN).send().await?.text().await?;

        let sapisid = extract_between(&config.cookie, "SAPISID=", ";")
            .ok_or_else(|| YError::InvalidCookie)?;

        let api_key = extract_between(&response_text, "INNERTUBE_API_KEY\":\"", "\"")
            .ok_or_else(|| YError::InvalidCookie)?;

        let client_version = extract_between(&response_text, "INNERTUBE_CLIENT_VERSION\":\"", "\"")
            .ok_or_else(|| YError::InvalidCookie)?;

        Ok(Self {
            http,
            sapisid: sapisid.to_string(),
            innertube_api_key: api_key.to_string(),
            client_version: client_version.to_string(),
            app_config: config,
        })
    }
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

    pub fn get_api_headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert("Content-Type", HeaderValue::from_static("application/json"));
        headers.insert("X-Origin", HeaderValue::from_static(YTM_DOMAIN));
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

        // Body chuẩn của InnerTube API để lấy dữ liệu trang chủ
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
            .json::<Value>() // Parse toàn bộ response thành Value để dễ truy vấn
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

fn extract_between(source: &str, start: &str, end: &str) -> Option<String> {
    source.find(start).and_then(|start_idx| {
        let start_pos = start_idx + start.len();
        source[start_pos..]
            .find(end)
            .map(|end_idx| source[start_pos..start_pos + end_idx].to_string())
    })
}
