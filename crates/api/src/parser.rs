use data::{PlayList, Song};
use serde_json::Value;
use error::{YError,Result};


// EXTRACT PLAYLISTS/ALBUMS FROM RESPONSED DATA (JSON TYPE)
pub fn extract_lists(data: Value) -> Result<(Vec<PlayList>, Vec<PlayList>, Option<String>)> {
    let mut albums: Vec<PlayList> = Vec::new();
    let mut playlists: Vec<PlayList> = Vec::new();

    let grid_renderer = data
        .pointer("/contents/singleColumnBrowseResultsRenderer/tabs/0/tabRenderer/content/sectionListRenderer/contents/0/gridRenderer")
        .or_else(|| data.pointer("/continuationContents/gridContinuation"))
        .ok_or(YError::InvalidCookie)?; 

    if let Some(items) = grid_renderer.get("items").and_then(|v| v.as_array()) {
        for item in items {
            if let Some(rerender) = item.get("musicTwoRowItemRenderer") {
                let page_type = rerender.pointer("/navigationEndpoint/browseEndpoint/browseEndpointContextSupportedConfigs/browseEndpointContextMusicConfig/pageType");
                if let Some(page_type_str) = page_type.and_then(|p| p.as_str()) {
                    let album = PlayList {
                        title: rerender.pointer("/title/runs/0/text").and_then(|v| v.as_str()).unwrap_or("Unknown").to_string(),
                        artist: rerender.pointer("/subtitle/runs/2/text").and_then(|v| v.as_str()).unwrap_or("Unknown").to_string(),
                        browse_id: rerender.pointer("/navigationEndpoint/browseEndpoint/browseId").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        playlist_id: rerender.pointer("/thumbnailOverlay/musicItemThumbnailOverlayRenderer/content/musicPlayButtonRenderer/playNavigationEndpoint/watchPlaylistEndpoint/playlistId")
                            .and_then(|v| v.as_str())
                            .unwrap_or("").to_string(),
                    };
                    match page_type_str {
                        "MUSIC_PAGE_TYPE_ALBUM" => albums.push(album),
                        "MUSIC_PAGE_TYPE_PLAYLIST" => playlists.push(album),
                        _ => {}
                    }
                }
            }
        }
    }
    let continuation_token = grid_renderer
        .pointer("/continuations/0/nextContinuationData/continuation")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    Ok((albums, playlists, continuation_token))
}

// EXTRACT SONGS FROM RESPONSED DATA (JSON TYPE)
pub fn extract_songs(data: Value) -> Result<Vec<Song>> {
    let mut songs = Vec::new();

    let track_list = data.pointer("/contents/twoColumnBrowseResultsRenderer/secondaryContents/sectionListRenderer/contents/0/musicShelfRenderer/contents")
        .or_else(|| data.pointer("/contents/twoColumnBrowseResultsRenderer/secondaryContents/sectionListRenderer/contents/0/musicPlaylistShelfRenderer/contents"))
        .and_then(|v| v.as_array())
        // Nếu không tìm thấy danh sách bài hát, báo lỗi phân tích dữ liệu ngay
        .ok_or(YError::InvalidCookie)?; 

    for item in track_list {
        if let Some(renderer) = item.get("musicResponsiveListItemRenderer") {
            let song = Song {
                title: renderer.pointer("/flexColumns/0/musicResponsiveListItemFlexColumnRenderer/text/runs/0/text")
                    .and_then(|v| v.as_str()).unwrap_or("Unknown").to_string(),
                
                video_id: renderer.pointer("/flexColumns/0/musicResponsiveListItemFlexColumnRenderer/text/runs/0/navigationEndpoint/watchEndpoint/videoId")
                    .or_else(|| renderer.pointer("/playlistItemData/videoId"))
                    .and_then(|v| v.as_str()).unwrap_or("").to_string(),
            };
            if !song.video_id.is_empty() {
                songs.push(song);
            }
        }
    }
    
    Ok(songs)
}

pub fn extract_search_albums(data: Value) -> Result<Vec<PlayList>> {
    let mut albums: Vec<PlayList> = Vec::new();
    let contents = data
        .pointer("/contents/tabbedSearchResultsRenderer/tabs/0/tabRenderer/content/sectionListRenderer/contents/0/musicShelfRenderer/contents")
        .and_then(|v| v.as_array())
        .ok_or(YError::InvalidCookie)?;

    for item in contents {
        if let Some(renderer) = item.get("musicResponsiveListItemRenderer") {
            let title = renderer
                .pointer("/flexColumns/0/musicResponsiveListItemFlexColumnRenderer/text/runs/0/text")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown")
                .to_string();
            let artist = renderer
                .pointer("/flexColumns/1/musicResponsiveListItemFlexColumnRenderer/text/runs/2/text")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown")
                .to_string();
            let browse_id = renderer
                .pointer("/navigationEndpoint/browseEndpoint/browseId")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let playlist_id = renderer
                .pointer("/overlay/musicItemThumbnailOverlayRenderer/content/musicPlayButtonRenderer/playNavigationEndpoint/watchPlaylistEndpoint/playlistId")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            if !browse_id.is_empty() {
                albums.push(PlayList {
                    title,
                    artist,
                    browse_id,
                    playlist_id,
                });
            }
        }
    }
    Ok(albums)
}
