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
pub fn extract_albums(data: &Value) -> (Vec<PlayList>, Vec<PlayList>) {
    let mut albums: Vec<PlayList> = Vec::new();
    let mut playlists: Vec<PlayList> = Vec::new();
    if let Some(items)= data.pointer("/contents/singleColumnBrowseResultsRenderer/tabs/0/tabRenderer/content/sectionListRenderer/contents/0/gridRenderer/items").and_then(|v| v.as_array()) {
            for item in items {
                if let Some(rerender) = item.get("musicTwoRowItemRenderer") {
                    let page_type = rerender.pointer("/title/runs/0/navigationEndpoint/browseEndpoint/browseEndpointContextSupportedConfigs/browseEndpointContextMusicConfig/pageType");
                    if let Some(page_type) = page_type.and_then(|p| p.as_str()) { 
                        let album = PlayList {
                            title: rerender.pointer("/title/runs/0/text").and_then(|v| v.as_str()).unwrap_or("Unknown").to_string(),
                            artist: rerender.pointer("/subtitle/runs/2/text").and_then(|v| v.as_str()).unwrap_or("Unknown").to_string(),
                            browse_id: rerender.pointer("/navigationEndpoint/browseEndpoint/browseId").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                            playlist_id: rerender.pointer("/thumbnailOverlay/musicItemThumbnailOverlayRenderer/content/musicPlayButtonRenderer/playNavigationEndpoint/watchPlaylistEndpoint/playlistId")
                                .and_then(|v| v.as_str())
                                .unwrap_or("").to_string(),
                        };
                        match page_type {
                            "MUSIC_PAGE_TYPE_ALBUM" => {albums.push(album);},
                            "MUSIC_PAGE_TYPE_PLAYLIST" => {playlists.push(album);},
                            _=>{}
                        }
                    }
                }
            }
        }
    (albums, playlists)
}
#[derive(Debug)]
pub struct Song {
    pub title: String,
    pub video_id: String,
    pub duration: String,
}

pub fn extract_songs_from_album(data: &Value) -> Vec<Song> {
    let mut songs = Vec::new();

    let items = data.pointer("/contents/twoColumnBrowseResultsRenderer/secondaryContents/sectionListRenderer/contents/0/musicShelfRenderer/contents")
        .and_then(|v| v.as_array());

    if let Some(track_list) = items {
        for item in track_list {
            if let Some(renderer) = item.get("musicResponsiveListItemRenderer") {
                let song = Song {
                    // Tiêu đề bài hát
                    title: renderer.pointer("/flexColumns/0/musicResponsiveListItemFlexColumnRenderer/text/runs/0/text")
                        .and_then(|v| v.as_str()).unwrap_or("Unknown").to_string(),
                    
                    // Video ID để phát nhạc
                    video_id: renderer.pointer("/flexColumns/0/musicResponsiveListItemFlexColumnRenderer/text/runs/0/navigationEndpoint/watchEndpoint/videoId")
                        .or_else(|| renderer.pointer("/playlistItemData/videoId"))
                        .and_then(|v| v.as_str()).unwrap_or("").to_string(),

                    // Thời lượng (ví dụ: 4:28)
                    duration: renderer.pointer("/fixedColumns/0/musicResponsiveListItemFixedColumnRenderer/text/runs/0/text")
                        .and_then(|v| v.as_str()).unwrap_or("0:00").to_string(),
               };
                songs.push(song);
            }
        }
    }
    songs
}
