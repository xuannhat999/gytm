use data::{PlayList, Song};
use serde_json::Value;
use error::{YError,YResult};


// EXTRACT PLAYLISTS/ALBUMS FROM RESPONSED DATA (JSON TYPE)
pub fn parse_lists(data: Value) -> YResult<(Vec<PlayList>, Vec<PlayList>, Option<String>)> {
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
                        is_saved: true
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
pub fn parse_songs(data: Value) -> YResult<Vec<Song>> {
    let mut songs = Vec::new();

    let track_list = data.pointer("/contents/twoColumnBrowseResultsRenderer/secondaryContents/sectionListRenderer/contents/0/musicShelfRenderer/contents")
        .or_else(|| data.pointer("/contents/twoColumnBrowseResultsRenderer/secondaryContents/sectionListRenderer/contents/0/musicPlaylistShelfRenderer/contents"))
        .and_then(|v| v.as_array())
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

pub fn parse_search_albums(data: Value) -> YResult<Vec<PlayList>> {
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

            let mut is_saved = false;
            if let Some(items) = renderer.pointer("/menu/menuRenderer/items").and_then(|v| v.as_array()) {
                for menu_item in items {
                    if let Some(toggle) = menu_item.get("toggleMenuServiceItemRenderer") {
                        if let Some(status) = toggle.pointer("/defaultServiceEndpoint/likeEndpoint/status").and_then(|v| v.as_str()) {
                            if status == "INDIFFERENT" {
                                is_saved = true;
                            }
                        }
                    }
                }
            }
            if !browse_id.is_empty() {
                albums.push(PlayList {
                    title,
                    artist,
                    browse_id,
                    playlist_id,
                    is_saved,
                });
            }
        }
    }
    Ok(albums)
}

pub fn parse_search_songs(data: Value) -> YResult<Vec<Song>> {
    let mut songs = Vec::new();
    if let Some(contents) = data["contents"]["tabbedSearchResultsRenderer"]["tabs"][0]["tabRenderer"]["content"]["sectionListRenderer"]["contents"].as_array() {
        for section in contents {
            if let Some(items) = section["musicShelfRenderer"]["contents"].as_array() {
                for item in items {
                    if let Some(renderer) = item.get("musicResponsiveListItemRenderer") {
                        let title = renderer["flexColumns"][0]["musicResponsiveListItemFlexColumnRenderer"]["text"]["runs"][0]["text"]
                            .as_str()
                            .unwrap_or("Unknown")
                            .to_string();

                        let video_id = renderer["overlay"]["musicItemThumbnailOverlayRenderer"]["content"]["musicPlayButtonRenderer"]["playNavigationEndpoint"]["watchEndpoint"]["videoId"]
                            .as_str()
                            .unwrap_or("")
                            .to_string();

                        if !video_id.is_empty() {
                            songs.push(Song {
                                video_id,
                                title,
                            });
                        }
                    }
                }
            }
        }
    }
    Ok(songs)
}

pub fn parse_params(data: Value) -> YResult<String> {
    let params = data
        .pointer("/contents/singleColumnMusicWatchNextResultsRenderer/tabbedRenderer/watchNextTabbedResultsRenderer/tabs/0/tabRenderer/content/musicQueueRenderer/content/playlistPanelRenderer/contents/1/automixPreviewVideoRenderer/content/automixPlaylistVideoRenderer/navigationEndpoint/watchPlaylistEndpoint/params")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string()); 
    params.ok_or(YError::InvalidCookie) 
}

pub fn parse_related_songs(data: Value) -> YResult<Vec<Song>> {
    let mut songs = Vec::new();
    if let Some(contents) = data
        .get("contents")
        .and_then(|c| c.get("singleColumnMusicWatchNextResultsRenderer"))
        .and_then(|s| s.get("tabbedRenderer"))
        .and_then(|t| t.get("watchNextTabbedResultsRenderer"))
        .and_then(|w| w.get("tabs"))
        .and_then(|tabs| tabs.as_array())
        .and_then(|arr| arr.first())
        .and_then(|tab| tab.get("tabRenderer"))
        .and_then(|tr| tr.get("content"))
        .and_then(|c| c.get("musicQueueRenderer"))
        .and_then(|mq| mq.get("content"))
        .and_then(|c| c.get("playlistPanelRenderer"))
        .and_then(|pp| pp.get("contents"))
        .and_then(|c| c.as_array())
    {
        for item in contents {
            if let Some(video) = item.get("playlistPanelVideoRenderer") {
                let video_id = video.get("videoId").and_then(|v| v.as_str());
                let title = video
                    .get("title")
                    .and_then(|t| t.get("runs"))
                    .and_then(|r| r.as_array())
                    .and_then(|arr| arr.first())
                    .and_then(|run| run.get("text"))
                    .and_then(|t| t.as_str());
                if let (Some(vid), Some(t)) = (video_id, title) {
                    songs.push(Song {
                        video_id: vid.to_string(),
                        title: t.to_string(),
                    });
                }
            }
        }
    }
    Ok(songs)
}
