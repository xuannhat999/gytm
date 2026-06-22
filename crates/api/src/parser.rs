use data::{Playlist, Song};
use error::{YError, YResult};
use serde_json::Value;

// EXTRACT PLAYLISTS/ALBUMS FROM RESPONSED DATA (JSON TYPE)
pub fn parse_lists(data: Value) -> YResult<(Vec<Playlist>, Vec<Playlist>, Option<String>)> {
    let mut albums: Vec<Playlist> = Vec::new();
    let mut playlists: Vec<Playlist> = Vec::new();

    let grid_renderer = data
        .pointer("/contents/singleColumnBrowseResultsRenderer/tabs/0/tabRenderer/content/sectionListRenderer/contents/0/gridRenderer")
        .or_else(|| data.pointer("/continuationContents/gridContinuation"))
        .ok_or(YError::InvalidResponse("Browse Library".to_string()))?;

    if let Some(items) = grid_renderer.get("items").and_then(|v| v.as_array()) {
        for item in items {
            if let Some(rerender) = item.get("musicTwoRowItemRenderer") {
                let page_type = rerender.pointer("/navigationEndpoint/browseEndpoint/browseEndpointContextSupportedConfigs/browseEndpointContextMusicConfig/pageType");
                if let Some(page_type_str) = page_type.and_then(|p| p.as_str()) {
                    let is_custom = rerender
                        .pointer("/menu/menuRenderer/items")
                        .and_then(|v| v.as_array())
                        .is_some_and(|items| {
                            items.iter().any(|item| {
                                item.pointer("/menuNavigationItemRenderer/navigationEndpoint/confirmDialogEndpoint/content/confirmDialogRenderer/confirmButton/buttonRenderer/serviceEndpoint/deletePlaylistEndpoint").is_some()
                                || item.pointer("/menuNavigationItemRenderer/navigationEndpoint/playlistEditorEndpoint").is_some()
                            })
                        });

                    let album = Playlist {
                        title: rerender.pointer("/title/runs/0/text").and_then(|v| v.as_str()).unwrap_or("Unknown").to_string(),
                        artist: rerender.pointer("/subtitle/runs/2/text").and_then(|v| v.as_str()).unwrap_or("Unknown").to_string(),
                        browse_id: rerender.pointer("/navigationEndpoint/browseEndpoint/browseId").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        playlist_id: rerender.pointer("/thumbnailOverlay/musicItemThumbnailOverlayRenderer/content/musicPlayButtonRenderer/playNavigationEndpoint/watchPlaylistEndpoint/playlistId")
                            .and_then(|v| v.as_str())
                            .unwrap_or("").to_string(),
                        is_saved: true,
                        is_custom,
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
        .ok_or(YError::InvalidResponse("Browse songs".to_string()))?;

    for item in track_list {
        if let Some(renderer) = item.get("musicResponsiveListItemRenderer") {
            let song = Song {
                title: renderer.pointer("/flexColumns/0/musicResponsiveListItemFlexColumnRenderer/text/runs/0/text")
                    .and_then(|v| v.as_str()).unwrap_or("Unknown").to_string(),

                set_video_id: renderer.pointer("/overlay/musicItemThumbnailOverlayRenderer/content/musicPlayButtonRenderer/playNavigationEndpoint/watchEndpoint/playlistSetVideoId")
                    .and_then(|v| v.as_str()).unwrap_or("").to_string(),
                video_id: renderer.pointer("/flexColumns/0/musicResponsiveListItemFlexColumnRenderer/text/runs/0/navigationEndpoint/watchEndpoint/videoId")
                    .or_else(|| renderer.pointer("/playlistItemData/videoId"))
                    .and_then(|v| v.as_str()).unwrap_or("").to_string(),
                duration: renderer.pointer("/fixedColumns/0/musicResponsiveListItemFixedColumnRenderer/text/runs/0/text").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            };
            if !song.video_id.is_empty() {
                songs.push(song);
            }
        }
    }

    Ok(songs)
}

pub fn parse_created_playlist(data: Value) -> YResult<Playlist> {
    let playlist_id = data["playlistId"]
        .as_str()
        .ok_or(YError::InvalidResponse("Create playlist".to_string()))?
        .to_string();

    let renderer = data["actions"][1]
        .get("handlePlaylistCreationCommand")
        .and_then(|h| h.get("createdPlaylist"))
        .and_then(|c| c.get("musicTwoRowItemRenderer"))
        .ok_or(YError::InvalidResponse("Create playlist".to_string()))?;

    let title = renderer["title"]["runs"][0]["text"]
        .as_str()
        .unwrap_or("Unknown")
        .to_string();

    let artist = renderer["subtitle"]["runs"][0]["text"]
        .as_str()
        .unwrap_or("Unknown")
        .to_string();

    let browse_id = renderer
        .pointer("/navigationEndpoint/browseEndpoint/browseId")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    Ok(Playlist {
        title,
        artist,
        browse_id,
        playlist_id,
        is_saved: true,
        is_custom: true,
    })
}

pub fn parse_search_albums(data: Value) -> YResult<Vec<Playlist>> {
    let mut albums: Vec<Playlist> = Vec::new();
    let contents = data
        .pointer("/contents/tabbedSearchResultsRenderer/tabs/0/tabRenderer/content/sectionListRenderer/contents/0/musicShelfRenderer/contents")
        .and_then(|v| v.as_array())
        .ok_or(YError::InvalidResponse("Search albums".to_string()))?;

    for item in contents {
        if let Some(renderer) = item.get("musicResponsiveListItemRenderer") {
            let title = renderer
                .pointer(
                    "/flexColumns/0/musicResponsiveListItemFlexColumnRenderer/text/runs/0/text",
                )
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown")
                .to_string();
            let artist = renderer
                .pointer(
                    "/flexColumns/1/musicResponsiveListItemFlexColumnRenderer/text/runs/2/text",
                )
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
            if let Some(items) = renderer
                .pointer("/menu/menuRenderer/items")
                .and_then(|v| v.as_array())
            {
                for menu_item in items {
                    if let Some(toggle) = menu_item.get("toggleMenuServiceItemRenderer") {
                        if let Some(status) = toggle
                            .pointer("/defaultServiceEndpoint/likeEndpoint/status")
                            .and_then(|v| v.as_str())
                        {
                            if status == "INDIFFERENT" {
                                is_saved = true;
                            }
                        }
                    }
                }
            }
            if !browse_id.is_empty() {
                albums.push(Playlist {
                    title,
                    artist,
                    browse_id,
                    playlist_id,
                    is_saved,
                    is_custom: false,
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

                        let mut duration = String::new();
                        if let Some(runs) = renderer.pointer("/flexColumns/1/musicResponsiveListItemFlexColumnRenderer/text/runs").and_then(|v| v.as_array()) {
                            if let Some(last_run_text) = runs.last().and_then(|r| r["text"].as_str()) {
                                if last_run_text.contains(':') {
                                    duration = last_run_text.trim().to_string();
                                }
                            }
                        }
                        if !video_id.is_empty() {
                            songs.push(Song {
                                video_id,
                                set_video_id: renderer["overlay"]["musicItemThumbnailOverlayRenderer"]["content"]["musicPlayButtonRenderer"]["playNavigationEndpoint"]["watchEndpoint"]["playlistSetVideoId"]
                                    .as_str()
                                    .unwrap_or("")
                                    .to_string(),
                                title,
                                duration,
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
    params.ok_or(YError::InvalidResponse("Get Params".to_string()))
}
pub fn parse_related_songs(data: Value) -> YResult<Vec<Song>> {
    let mut songs = Vec::new();
    let contents = data.pointer("/contents/singleColumnMusicWatchNextResultsRenderer/tabbedRenderer/watchNextTabbedResultsRenderer/tabs/0/tabRenderer/content/musicQueueRenderer/content/playlistPanelRenderer/contents").and_then(|c|c.as_array()).ok_or(YError::InvalidResponse("Get related songs".to_string()))?;
    for item in contents {
        if let Some(video) = item.get("playlistPanelVideoRenderer") {
            let video_id = video.get("videoId").and_then(|v| v.as_str());
            let title = video.pointer("/title/runs/0/text").and_then(|v| v.as_str());
            let duration = video
                .pointer("/lengthText/runs/0/text")
                .and_then(|v| v.as_str());

            if let (Some(vid), Some(t), Some(duration)) = (video_id, title, duration) {
                songs.push(Song {
                    video_id: vid.to_string(),
                    set_video_id: "".to_string(),
                    title: t.to_string(),
                    duration: duration.to_string(),
                });
            }
        }
    }
    Ok(songs)
}
