use data::app::{PlayListPrivacy, Song};
use error::{YError, YResult};
use reqwest::{
    Client,
    cookie::Jar,
    header::{HeaderMap, HeaderValue},
};
use state::client_state::ClientState;
use std::sync::Arc;

use crate::{
    client,
    request::{
        ActionsContent, BrowseIdRequest, CreatePlaylistRequest, EmptyRequest,
        GetContinuationRequest, GetRelatedSongsRequest, PlaylistIdRequest, QueryRequest,
        QueryWithParamsRequest, RequestClient, RequestContext, SaveAlbumRequest,
        SaveUnsaveListRequest, TargetContent, TargetRequest, VideoIdRequest,
    },
};

pub struct YTDao {
    pub http: Client,
    pub sapisid: Option<String>,
    pub innertube_api_key: String,
    pub client_version: String,
    pub auth_user: usize,
}

pub static YTM_DOMAIN: &str = "https://music.youtube.com";
const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

impl YTDao {
    pub async fn default() -> YResult<Self> {
        let (http, innertube_api_key, client_version) = build_client_fields(Jar::default()).await?;
        Ok(Self {
            http,
            sapisid: None,
            innertube_api_key,
            client_version,
            auth_user: 0,
        })
    }

    pub async fn reload(&mut self, jar: Jar, sapisid: String, auth_user: usize) -> YResult<()> {
        let (http, innertube_api_key, client_version) = build_client_fields(jar).await?;
        self.http = http;
        self.sapisid = Some(sapisid);
        self.innertube_api_key = innertube_api_key;
        self.client_version = client_version;
        self.auth_user = auth_user;
        Ok(())
    }

    pub async fn new(client_state: &ClientState) -> YResult<Self> {
        if let Ok((browser, profile, gecko_container, account)) =
            client_state.get_validated_fields()
        {
            if let Ok((jar, sapisid)) = client::load_cookies(browser, profile, gecko_container) {
                let (http, innertube_api_key, client_version) = build_client_fields(jar).await?;

                return Ok(Self {
                    http,
                    sapisid: Some(sapisid),
                    innertube_api_key,
                    client_version,
                    auth_user: account.map_or(0, |a| a.auth_user),
                });
            }
        }
        Self::default().await
    }

    // This function is adapted from: https://github.com/ccgauche/ytermusic.git
    // Original source: https://github.com/ccgauche/ytermusic/blob/master/crates/ytpapi2/src/lib.rs
    fn compute_sapi_hash(&self, sapisid: &str) -> String {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut hasher = sha1_smol::Sha1::new();
        let message = format!("{timestamp} {} {YTM_DOMAIN}", sapisid);
        hasher.update(message.as_bytes());
        let result = hasher.digest();
        let hex_hash = result.to_string();
        format!("{}_{}", timestamp, hex_hash)
    }

    // This function is adapted from: https://github.com/ccgauche/ytermusic.git
    // Original source: https://github.com/ccgauche/ytermusic/blob/master/crates/ytpapi2/src/lib.rs
    pub fn get_api_headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert("Content-Type", HeaderValue::from_static("application/json"));
        headers.insert("Origin", HeaderValue::from_static(YTM_DOMAIN));
        headers.insert(
            "X-Goog-AuthUser",
            HeaderValue::from_str(&self.auth_user.to_string()).unwrap(),
        );
        if let Some(ref sapisid) = self.sapisid {
            let auth_val = format!("SAPISIDHASH {}", self.compute_sapi_hash(sapisid));
            headers.insert("Authorization", HeaderValue::from_str(&auth_val).unwrap());
        }
        headers
    }

    pub fn get_api_headers_with_user_auth(&self, usr_auth: usize) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert("Content-Type", HeaderValue::from_static("application/json"));
        headers.insert("Origin", HeaderValue::from_static(YTM_DOMAIN));
        headers.insert(
            "X-Goog-AuthUser",
            HeaderValue::from_str(&usr_auth.to_string()).unwrap(),
        );
        if let Some(ref sapisid) = self.sapisid {
            let auth_val = format!("SAPISIDHASH {}", self.compute_sapi_hash(sapisid));
            headers.insert("Authorization", HeaderValue::from_str(&auth_val).unwrap());
        }
        headers
    }
    fn api_url(&self, endpoint: &str) -> String {
        format!(
            "{}/youtubei/v1/{}?key={}&alt=json",
            YTM_DOMAIN, endpoint, self.innertube_api_key
        )
    }

    fn get_context(&self) -> RequestContext<'_> {
        RequestContext {
            client: RequestClient {
                client_name: "WEB_REMIX",
                client_version: &self.client_version,
            },
        }
    }

    pub async fn get_raw_lists(&self) -> YResult<String> {
        let url = self.api_url("browse");

        let body = BrowseIdRequest {
            context: self.get_context(),
            browse_id: "FEmusic_library_landing",
        };
        let response = self
            .http
            .post(&url)
            .headers(self.get_api_headers())
            .json(&body)
            .send()
            .await?
            .text()
            .await?;
        Ok(response)
    }

    pub async fn get_account_email_from_idx(&self, user_auth: usize) -> YResult<String> {
        let url = self.api_url("account/accounts_list");
        let body = EmptyRequest {
            context: self.get_context(),
        };
        let response = self
            .http
            .post(&url)
            .headers(self.get_api_headers_with_user_auth(user_auth))
            .json(&body)
            .send()
            .await?
            .text()
            .await?;
        Ok(response)
    }

    pub async fn get_continuation_raw(&self, token: &str) -> YResult<String> {
        let url = self.api_url("browse");
        let body = GetContinuationRequest {
            context: self.get_context(),
            continuation: token,
        };
        let response = self
            .http
            .post(&url)
            .headers(self.get_api_headers())
            .json(&body)
            .send()
            .await?
            .text()
            .await?;

        Ok(response)
    }
    pub async fn get_songs_raw(&self, browse_id: &str) -> YResult<String> {
        let url = self.api_url("browse");
        let body = BrowseIdRequest {
            context: self.get_context(),
            browse_id,
        };
        let text = self
            .http
            .post(&url)
            .headers(self.get_api_headers())
            .json(&body)
            .send()
            .await?
            .text()
            .await?;
        Ok(text)
    }

    pub async fn search_with_params_raw(&self, query: &str, rtype: u8) -> YResult<String> {
        let params = if rtype == 1 {
            "EgWKAQIIAWoMEAQQAxAFEAkQEBAK" // SONG
        } else {
            "EgWKAQIYAWoMEAQQAxAFEAkQEBAK" // ALBUM
        };
        let url = self.api_url("search");

        let body = QueryWithParamsRequest {
            context: self.get_context(),
            query,
            params,
        };
        let response = self
            .http
            .post(&url)
            .headers(self.get_api_headers())
            .json(&body)
            .send()
            .await?
            .text()
            .await?;
        Ok(response)
    }
    pub async fn search_raw(&self, query: &str) -> YResult<String> {
        let body = QueryRequest {
            context: self.get_context(),
            query,
        };
        let url = self.api_url("search");
        let response = self
            .http
            .post(&url)
            .headers(self.get_api_headers())
            .json(&body)
            .send()
            .await?
            .text()
            .await?;
        Ok(response)
    }
    pub async fn get_params_raw(&self, video_id: &str) -> YResult<String> {
        let url = self.api_url("next");
        let body = VideoIdRequest {
            context: self.get_context(),
            video_id,
        };
        let response = self
            .http
            .post(&url)
            .headers(self.get_api_headers())
            .json(&body)
            .send()
            .await?
            .text()
            .await?;
        Ok(response)
    }

    pub async fn create_playlist_raw(
        &self,
        title: &str,
        desc: &str,
        privacy: PlayListPrivacy,
    ) -> YResult<String> {
        let url = self.api_url("playlist/create");
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
            .headers(self.get_api_headers())
            .json(&body)
            .send()
            .await?
            .text()
            .await?;
        Ok(response)
    }

    pub async fn get_related_songs_raw(&self, playlist_id: &str, params: &str) -> YResult<String> {
        let url = self.api_url("next");
        let body = GetRelatedSongsRequest {
            context: self.get_context(),
            playlist_id,
            params,
            tuner_setting_value: "AUTOMIX_SETTING_NORMAL",
        };
        let response = self
            .http
            .post(&url)
            .headers(self.get_api_headers())
            .json(&body)
            .send()
            .await?
            .text()
            .await?;
        Ok(response)
    }

    pub async fn save_album_raw(&self, playlist_id: &str) -> YResult<()> {
        let url = self.api_url("like/like");
        let body = SaveAlbumRequest {
            context: self.get_context(),
            target: TargetContent {
                playlist_id: Some(playlist_id),
                video_id: None,
            },
            status: "LIKE",
        };

        let status = self
            .http
            .post(&url)
            .headers(self.get_api_headers())
            .json(&body)
            .send()
            .await?
            .status()
            .is_success();
        if status {
            Ok(())
        } else {
            Err(YError::BadStatus(String::from("Unsave custom playlist")))
        }
    }

    pub async fn unsave_cus_playlist_raw(&self, playlist_id: &str) -> YResult<()> {
        let url = self.api_url("playlist/delete");

        let body = PlaylistIdRequest {
            context: self.get_context(),
            playlist_id,
        };
        let status = self
            .http
            .post(&url)
            .headers(self.get_api_headers())
            .json(&body)
            .send()
            .await?
            .status()
            .is_success();
        if status {
            Ok(())
        } else {
            Err(YError::BadStatus(String::from("Unsave custom playlist")))
        }
    }

    pub async fn unsave_album_raw(&self, playlist_id: &str) -> YResult<()> {
        let url = self.api_url("like/removelike");
        let body = TargetRequest {
            context: self.get_context(),
            target: TargetContent {
                playlist_id: Some(playlist_id),
                video_id: None,
            },
        };

        let status = self
            .http
            .post(&url)
            .headers(self.get_api_headers())
            .json(&body)
            .send()
            .await?
            .status()
            .is_success();
        if status {
            Ok(())
        } else {
            Err(YError::BadStatus(String::from("Unsave album")))
        }
    }

    pub async fn save_to_playlist_raw(&self, song: &Song, playlist_id: &str) -> YResult<()> {
        let url = self.api_url("browse/edit_playlist");
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
            .headers(self.get_api_headers())
            .json(&body)
            .send()
            .await?;
        let status = response.status();
        if !status.is_success() {
            Err(YError::BadStatus(String::from("Save song to playlist")))
        } else {
            let text = response.text().await?;
            if text.contains("STATUS_SUCCEEDED") {
                Ok(())
            } else {
                Err(YError::AlreadyInPlaylist)
            }
        }
    }

    pub async fn unsave_to_playlist_raw(&self, song: &Song, playlist_id: &str) -> YResult<()> {
        let video_id = &song.video_id;
        let set_video_id = &song.set_video_id;
        let url = self.api_url("browse/edit_playlist");
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
            .headers(self.get_api_headers())
            .json(&body)
            .send()
            .await?;
        let status = response.status();
        if !status.is_success() {
            Err(YError::BadStatus(String::from("Unsave song to playlist")))
        } else {
            let text = response.text().await?;
            if text.contains("STATUS_SUCCEEDED") {
                Ok(())
            } else {
                Err(YError::AlreadyInPlaylist)
            }
        }
    }

    pub async fn unlike_song_raw(&self, song: &Song) -> YResult<()> {
        let video_id = &song.video_id;
        let url = self.api_url("like/removelike");
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
            .headers(self.get_api_headers())
            .json(&body)
            .send()
            .await?;

        let status = response.status();
        if status.is_success() {
            Ok(())
        } else {
            Err(YError::BadStatus(String::from("Unlike Song")))
        }
    }

    pub async fn like_song_raw(&self, song: &Song) -> YResult<()> {
        let video_id = &song.video_id;
        let url = self.api_url("like/like");

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
            .headers(self.get_api_headers())
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
}
async fn build_client_fields(jar: Jar) -> YResult<(Client, String, String)> {
    let http = Client::builder()
        .cookie_provider(Arc::new(jar))
        .user_agent(USER_AGENT)
        .build()?;

    let response_text = http.get(YTM_DOMAIN).send().await?.text().await?;
    let innertube_api_key = extract_between(&response_text, "INNERTUBE_API_KEY\":\"", "\"")
        .ok_or_else(|| YError::InvalidCookie)?;

    let client_version = extract_between(&response_text, "INNERTUBE_CLIENT_VERSION\":\"", "\"")
        .ok_or_else(|| YError::InvalidCookie)?;
    Ok((http, innertube_api_key, client_version))
}

fn extract_between(source: &str, start: &str, end: &str) -> Option<String> {
    source.find(start).and_then(|start_idx| {
        let start_pos = start_idx + start.len();
        source[start_pos..]
            .find(end)
            .map(|end_idx| source[start_pos..start_pos + end_idx].to_string())
    })
}
