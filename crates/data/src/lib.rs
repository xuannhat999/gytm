use ::serde::Deserialize;
use error::{Result, YError};
use serde_json::Value;
use std::fs;

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub cookie: String,
    pub user_agent: String,
}

impl AppConfig {
    pub fn load() -> Result<Self> {
        let conf_file = dirs::config_dir()
            .ok_or(YError::ConfigFileErr)?
            .join("ytm/config.json");
        let content = fs::read_to_string(conf_file)?;
        let config: AppConfig = serde_json::from_str(&content)?;
        Ok(config)
    }
}
#[derive(Debug)]
pub struct PlayList {
    pub title: String,
    pub artist: String,
    pub browse_id: String,
    pub playlist_id: String,
}
pub fn extract_albums(data: &Value) -> Vec<PlayList> {
    let mut playlists: Vec<PlayList> = Vec::new();
    let allowed_types = ["MUSIC_PAGE_TYPE_ALBUM", "MUSIC_PAGE_TYPE_PLAYLIST"];
    if let Some(items)= data.pointer("/contents/singleColumnBrowseResultsRenderer/tabs/0/tabRenderer/content/sectionListRenderer/contents/0/gridRenderer/items").and_then(|v| v.as_array()) {
            for item in items {
                if let Some(rerender) = item.get("musicTwoRowItemRenderer") {
                    let page_type = rerender.pointer("/title/runs/0/navigationEndpoint/browseEndpoint/browseEndpointContextSupportedConfigs/browseEndpointContextMusicConfig/pageType");
                    if let Some(page_type) = page_type.and_then(|p| p.as_str()) && allowed_types.contains(&page_type) {
                        let playlist = PlayList {
                            title: rerender.pointer("/title/runs/0/text").and_then(|v| v.as_str()).unwrap_or("Unknown").to_string(),
                            artist: rerender.pointer("/subtitle/runs/2/text").and_then(|v| v.as_str()).unwrap_or("Unknown").to_string(),
                            browse_id: rerender.pointer("/navigationEndpoint/browseEndpoint/browseId").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                            playlist_id: rerender.pointer("/thumbnailOverlay/musicItemThumbnailOverlayRenderer/content/musicPlayButtonRenderer/playNavigationEndpoint/watchPlaylistEndpoint/playlistId")
                                .and_then(|v| v.as_str())
                                .unwrap_or("").to_string(),
                        };
                        playlists.push(playlist);
                    }
                }
            }
        }
    playlists
}
