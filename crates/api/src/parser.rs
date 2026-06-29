use data::{Playlist, Song};
use error::{YError, YResult};

// pub fn parse_lists(data: Value) -> YResult<(Vec<Playlist>, Vec<Playlist>, Option<String>)> {
//     let mut albums: Vec<Playlist> = Vec::new();
//     let mut playlists: Vec<Playlist> = Vec::new();
//
//     let grid_renderer = data
//         .pointer("/contents/singleColumnBrowseResultsRenderer/tabs/0/tabRenderer/content/sectionListRenderer/contents/0/gridRenderer")
//         .or_else(|| data.pointer("/continuationContents/gridContinuation"))
//         .ok_or(YError::InvalidResponse("Browse Library".to_string()))?;
//
//     if let Some(items) = grid_renderer.get("items").and_then(|v| v.as_array()) {
//         for item in items {
//             if let Some(rerender) = item.get("musicTwoRowItemRenderer") {
//                 let page_type = rerender.pointer("/navigationEndpoint/browseEndpoint/browseEndpointContextSupportedConfigs/browseEndpointContextMusicConfig/pageType");
//                 if let Some(page_type_str) = page_type.and_then(|p| p.as_str()) {
//                     let is_custom = rerender
//                         .pointer("/menu/menuRenderer/items")
//                         .and_then(|v| v.as_array())
//                         .is_some_and(|items| {
//                             items.iter().any(|item| {
//                                 item.pointer("/menuNavigationItemRenderer/navigationEndpoint/confirmDialogEndpoint/content/confirmDialogRenderer/confirmButton/buttonRenderer/serviceEndpoint/deletePlaylistEndpoint").is_some()
//                                 || item.pointer("/menuNavigationItemRenderer/navigationEndpoint/playlistEditorEndpoint").is_some()
//                             })
//                         });
//
//                     let album = Playlist {
//                         title: rerender.pointer("/title/runs/0/text").and_then(|v| v.as_str()).unwrap_or("Unknown").to_string(),
//                         artist: rerender.pointer("/subtitle/runs/2/text").and_then(|v| v.as_str()).unwrap_or("Unknown").to_string(),
//                         browse_id: rerender.pointer("/navigationEndpoint/browseEndpoint/browseId").and_then(|v| v.as_str()).unwrap_or("").to_string(),
//                         playlist_id: rerender.pointer("/thumbnailOverlay/musicItemThumbnailOverlayRenderer/content/musicPlayButtonRenderer/playNavigationEndpoint/watchPlaylistEndpoint/playlistId")
//                             .and_then(|v| v.as_str())
//                             .unwrap_or("").to_string(),
//                         is_saved: true,
//                         is_custom,
//                     };
//                     match page_type_str {
//                         "MUSIC_PAGE_TYPE_ALBUM" => albums.push(album),
//                         "MUSIC_PAGE_TYPE_PLAYLIST" => playlists.push(album),
//                         _ => {}
//                     }
//                 }
//             }
//         }
//     }
//     let continuation_token = grid_renderer
//         .pointer("/continuations/0/nextContinuationData/continuation")
//         .and_then(|v| v.as_str())
//         .map(|s| s.to_string());
//     Ok((albums, playlists, continuation_token))
// }

pub fn parse_lists(data: &str) -> YResult<(Vec<Playlist>, Vec<Playlist>, Option<String>)> {
    let mut albums: Vec<Playlist> = Vec::new();
    let mut playlists: Vec<Playlist> = Vec::new();

    let json = gjson::get(
        data,
        "contents.singleColumnBrowseResultsRenderer.tabs.0.tabRenderer.content.sectionListRenderer.contents.0.gridRenderer",
    );
    let json = if json.exists() {
        json
    } else {
        gjson::get(data, "continuationContents.gridContinuation")
    };
    if !json.exists() {
        return Err(YError::InvalidResponse("Browse Library".to_string()));
    }

    let items = json.get("items");
    if items.exists() {
        items.each(|_, item| {
            let renderer = item.get("musicTwoRowItemRenderer");
            if !renderer.exists() {
                return true;
            }

            let page_type_v = renderer.get("navigationEndpoint.browseEndpoint.browseEndpointContextSupportedConfigs.browseEndpointContextMusicConfig.pageType");
            let page_type = page_type_v.str();
            let mut is_custom = false;
            let menu = renderer.get("menu.menuRenderer.items");
            if menu.exists() {
                menu.each(|_, mi| {
                    let p1 = "menuNavigationItemRenderer.navigationEndpoint.confirmDialogEndpoint.content.confirmDialogRenderer.confirmButton.buttonRenderer.serviceEndpoint.deletePlaylistEndpoint";
                    let p2 = "menuNavigationItemRenderer.navigationEndpoint.playlistEditorEndpoint";
                    if mi.get(p1).exists() || mi.get(p2).exists() {
                        is_custom = true;
                        return false;
                    }
                    true
                });
            }

            let title_v = renderer.get("title.runs.0.text");
            let artist_v = renderer.get("subtitle.runs.2.text");
            let browse_id_v = renderer.get("navigationEndpoint.browseEndpoint.browseId");
            let playlist_id_v = renderer.get("thumbnailOverlay.musicItemThumbnailOverlayRenderer.content.musicPlayButtonRenderer.playNavigationEndpoint.watchPlaylistEndpoint.playlistId");

            let album = Playlist {
                title: if title_v.str().is_empty() { "Unknown".to_string() } else { title_v.str().to_string() },
                artist: if artist_v.str().is_empty() { "Unknown".to_string() } else { artist_v.str().to_string() },
                browse_id: browse_id_v.str().to_string(),
                playlist_id: playlist_id_v.str().to_string(),
                is_saved: true,
                is_custom,
            };

            match page_type {
                "MUSIC_PAGE_TYPE_ALBUM" => albums.push(album),
                "MUSIC_PAGE_TYPE_PLAYLIST" => playlists.push(album),
                _ => {}
            }
            true
        });
    }

    let token = json.get("continuations.0.nextContinuationData.continuation");
    let continuation_token = if token.exists() {
        Some(token.str().to_string())
    } else {
        None
    };

    Ok((albums, playlists, continuation_token))
}

// pub fn parse_songs(data: Value) -> YResult<Vec<Song>> {
//     let mut songs = Vec::new();
//
//     let track_list = data.pointer("/contents/twoColumnBrowseResultsRenderer/secondaryContents/sectionListRenderer/contents/0/musicShelfRenderer/contents")
//         .or_else(|| data.pointer("/contents/twoColumnBrowseResultsRenderer/secondaryContents/sectionListRenderer/contents/0/musicPlaylistShelfRenderer/contents"))
//         .and_then(|v| v.as_array())
//         .ok_or(YError::InvalidResponse("Browse songs".to_string()))?;
//
//     for item in track_list {
//         if let Some(renderer) = item.get("musicResponsiveListItemRenderer") {
//             let song = Song {
//                 title: renderer.pointer("/flexColumns/0/musicResponsiveListItemFlexColumnRenderer/text/runs/0/text")
//                     .and_then(|v| v.as_str()).unwrap_or("Unknown").to_string(),
//
//                 set_video_id: renderer.pointer("/overlay/musicItemThumbnailOverlayRenderer/content/musicPlayButtonRenderer/playNavigationEndpoint/watchEndpoint/playlistSetVideoId")
//                     .and_then(|v| v.as_str()).unwrap_or("").to_string(),
//                 video_id: renderer.pointer("/flexColumns/0/musicResponsiveListItemFlexColumnRenderer/text/runs/0/navigationEndpoint/watchEndpoint/videoId")
//                     .or_else(|| renderer.pointer("/playlistItemData/videoId"))
//                     .and_then(|v| v.as_str()).unwrap_or("").to_string(),
//                 duration: renderer.pointer("/fixedColumns/0/musicResponsiveListItemFixedColumnRenderer/text/runs/0/text").and_then(|v| v.as_str()).unwrap_or("").to_string(),
//             };
//             if !song.video_id.is_empty() {
//                 songs.push(song);
//             }
//         }
//     }
//     Ok(songs)
// }

pub fn parse_songs(data: &str) -> YResult<Vec<Song>> {
    let path = "contents.twoColumnBrowseResultsRenderer.secondaryContents.sectionListRenderer.contents.0.musicShelfRenderer.contents";
    let alt_path = "contents.twoColumnBrowseResultsRenderer.secondaryContents.sectionListRenderer.contents.0.musicPlaylistShelfRenderer";
    let alt_path_contents = "contents.twoColumnBrowseResultsRenderer.secondaryContents.sectionListRenderer.contents.0.musicPlaylistShelfRenderer.contents";
    let json = gjson::get(data, path);
    let json = if json.exists() {
        json
    } else if !gjson::get(data, alt_path).exists() {
        return Err(YError::InvalidResponse("Browse songs".to_string()));
    } else {
        let contents = gjson::get(data, alt_path_contents);
        if !contents.exists() {
            return Ok(Vec::new());
        }
        contents
    };
    let mut songs = Vec::new();
    json.each(|_, item| {
        let r = item.get("musicResponsiveListItemRenderer");
        if !r.exists() {
            return true;
        }
        let video_id = r.get("flexColumns.0.musicResponsiveListItemFlexColumnRenderer.text.runs.0.navigationEndpoint.watchEndpoint.videoId");
        let video_id = if video_id.exists() { video_id } else { r.get("playlistItemData.videoId") };
        if !video_id.str().is_empty() {
            let title = r.get("flexColumns.0.musicResponsiveListItemFlexColumnRenderer.text.runs.0.text");
            let set_video_id = r.get("overlay.musicItemThumbnailOverlayRenderer.content.musicPlayButtonRenderer.playNavigationEndpoint.watchEndpoint.playlistSetVideoId");
            let duration = r.get("fixedColumns.0.musicResponsiveListItemFixedColumnRenderer.text.runs.0.text");
            songs.push(Song {
                video_id: video_id.str().to_string(),
                set_video_id: set_video_id.str().to_string(),
                title: if title.str().is_empty() { "Unknown".to_string() } else { title.str().to_string() },
                duration: duration.str().to_string(),
            });
        }
        true
    });
    Ok(songs)
}

// pub fn parse_created_playlist(data: Value) -> YResult<Playlist> {
//     let playlist_id = data["playlistId"]
//         .as_str()
//         .ok_or(YError::InvalidResponse("Create playlist".to_string()))?
//         .to_string();
//
//     let renderer = data["actions"][1]
//         .get("handlePlaylistCreationCommand")
//         .and_then(|h| h.get("createdPlaylist"))
//         .and_then(|c| c.get("musicTwoRowItemRenderer"))
//         .ok_or(YError::InvalidResponse("Create playlist".to_string()))?;
//
//     let title = renderer["title"]["runs"][0]["text"]
//         .as_str()
//         .unwrap_or("Unknown")
//         .to_string();
//
//     let artist = renderer["subtitle"]["runs"][0]["text"]
//         .as_str()
//         .unwrap_or("Unknown")
//         .to_string();
//
//     let browse_id = renderer
//         .pointer("/navigationEndpoint/browseEndpoint/browseId")
//         .and_then(|v| v.as_str())
//         .unwrap_or("")
//         .to_string();
//
//     Ok(Playlist {
//         title,
//         artist,
//         browse_id,
//         playlist_id,
//         is_saved: true,
//         is_custom: true,
//     })
// }

pub fn parse_created_playlist(data: &str) -> YResult<Playlist> {
    let playlist_id = gjson::get(data, "playlistId").str().to_string();
    if playlist_id.is_empty() {
        return Err(YError::InvalidResponse("Create playlist".to_string()));
    }
    let renderer_path =
        "actions.1.handlePlaylistCreationCommand.createdPlaylist.musicTwoRowItemRenderer";
    let renderer = gjson::get(data, renderer_path);
    if playlist_id.is_empty() || !renderer.exists() {
        return Err(YError::InvalidResponse("Create playlist".to_string()));
    }
    let title = renderer.get("title.runs.0.text").str().to_string();
    let artist = renderer.get("subtitle.runs.0.text").str().to_string();
    let browse_id = renderer
        .get("navigationEndpoint.browseEndpoint.browseId")
        .str()
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

// pub fn parse_search_albums(data: Value) -> YResult<Vec<Playlist>> {
//     let mut albums: Vec<Playlist> = Vec::new();
//     let contents = data
//         .pointer("/contents/tabbedSearchResultsRenderer/tabs/0/tabRenderer/content/sectionListRenderer/contents/0/musicShelfRenderer/contents")
//         .and_then(|v| v.as_array())
//         .ok_or(YError::InvalidResponse("Search albums".to_string()))?;
//
//     for item in contents {
//         if let Some(renderer) = item.get("musicResponsiveListItemRenderer") {
//             let title = renderer
//                 .pointer(
//                     "/flexColumns/0/musicResponsiveListItemFlexColumnRenderer/text/runs/0/text",
//                 )
//                 .and_then(|v| v.as_str())
//                 .unwrap_or("Unknown")
//                 .to_string();
//             let artist = renderer
//                 .pointer(
//                     "/flexColumns/1/musicResponsiveListItemFlexColumnRenderer/text/runs/2/text",
//                 )
//                 .and_then(|v| v.as_str())
//                 .unwrap_or("Unknown")
//                 .to_string();
//             let browse_id = renderer
//                 .pointer("/navigationEndpoint/browseEndpoint/browseId")
//                 .and_then(|v| v.as_str())
//                 .unwrap_or("")
//                 .to_string();
//
//             let playlist_id = renderer
//                 .pointer("/overlay/musicItemThumbnailOverlayRenderer/content/musicPlayButtonRenderer/playNavigationEndpoint/watchPlaylistEndpoint/playlistId")
//                 .and_then(|v| v.as_str())
//                 .unwrap_or("")
//                 .to_string();
//
//             let mut is_saved = false;
//             if let Some(items) = renderer
//                 .pointer("/menu/menuRenderer/items")
//                 .and_then(|v| v.as_array())
//             {
//                 for menu_item in items {
//                     if let Some(toggle) = menu_item.get("toggleMenuServiceItemRenderer") {
//                         if let Some(status) = toggle
//                             .pointer("/defaultServiceEndpoint/likeEndpoint/status")
//                             .and_then(|v| v.as_str())
//                         {
//                             if status == "INDIFFERENT" {
//                                 is_saved = true;
//                             }
//                         }
//                     }
//                 }
//             }
//             if !browse_id.is_empty() {
//                 albums.push(Playlist {
//                     title,
//                     artist,
//                     browse_id,
//                     playlist_id,
//                     is_saved,
//                     is_custom: false,
//                 });
//             }
//         }
//     }
//     Ok(albums)
// }
//
pub fn parse_search_albums(data: &str) -> YResult<Vec<Playlist>> {
    let contents = gjson::get(
        data,
        "contents.tabbedSearchResultsRenderer.tabs.0.tabRenderer.content.sectionListRenderer.contents.0.musicShelfRenderer.contents",
    );
    if !contents.exists() {
        return Err(YError::InvalidResponse("Search albums".to_string()));
    }

    let mut albums = Vec::new();
    contents.each(|_, item| {
        let renderer = item.get("musicResponsiveListItemRenderer");
        if !renderer.exists() {
            return true;
        }

        let title_v = renderer.get("flexColumns.0.musicResponsiveListItemFlexColumnRenderer.text.runs.0.text");
        let artist_v = renderer.get("flexColumns.1.musicResponsiveListItemFlexColumnRenderer.text.runs.2.text");
        let browse_id_v = renderer.get("navigationEndpoint.browseEndpoint.browseId");
        let playlist_id_v = renderer.get("overlay.musicItemThumbnailOverlayRenderer.content.musicPlayButtonRenderer.playNavigationEndpoint.watchPlaylistEndpoint.playlistId");

        let mut is_saved = false;
        let menu = renderer.get("menu.menuRenderer.items");
        if menu.exists() {
            menu.each(|_, mi| {
                if mi.get("toggleMenuServiceItemRenderer.defaultServiceEndpoint.likeEndpoint.status").str() == "INDIFFERENT" {
                    is_saved = true;
                    return false;
                }
                true
            });
        }

        if !browse_id_v.str().is_empty() {
            albums.push(Playlist {
                title: if title_v.str().is_empty() { "Unknown".to_string() } else { title_v.str().to_string() },
                artist: if artist_v.str().is_empty() { "Unknown".to_string() } else { artist_v.str().to_string() },
                browse_id: browse_id_v.str().to_string(),
                playlist_id: playlist_id_v.str().to_string(),
                is_saved,
                is_custom: false,
            });
        }
        true
    });
    Ok(albums)
}

// pub fn parse_search_songs(data: Value) -> YResult<Vec<Song>> {
//     let mut songs = Vec::new();
//     if let Some(contents) = data["contents"]["tabbedSearchResultsRenderer"]["tabs"][0]["tabRenderer"]["content"]["sectionListRenderer"]["contents"].as_array() {
//         for section in contents {
//             if let Some(items) = section["musicShelfRenderer"]["contents"].as_array() {
//                 for item in items {
//                     if let Some(renderer) = item.get("musicResponsiveListItemRenderer") {
//                         let title = renderer["flexColumns"][0]["musicResponsiveListItemFlexColumnRenderer"]["text"]["runs"][0]["text"]
//                             .as_str()
//                             .unwrap_or("Unknown")
//                             .to_string();
//
//                         let video_id = renderer["overlay"]["musicItemThumbnailOverlayRenderer"]["content"]["musicPlayButtonRenderer"]["playNavigationEndpoint"]["watchEndpoint"]["videoId"]
//                             .as_str()
//                             .unwrap_or("")
//                             .to_string();
//
//                         let mut duration = String::new();
//                         if let Some(runs) = renderer.pointer("/flexColumns/1/musicResponsiveListItemFlexColumnRenderer/text/runs").and_then(|v| v.as_array()) {
//                             if let Some(last_run_text) = runs.last().and_then(|r| r["text"].as_str()) {
//                                 if last_run_text.contains(':') {
//                                     duration = last_run_text.trim().to_string();
//                                 }
//                             }
//                         }
//                         if !video_id.is_empty() {
//                             songs.push(Song {
//                                 video_id,
//                                 set_video_id: renderer["overlay"]["musicItemThumbnailOverlayRenderer"]["content"]["musicPlayButtonRenderer"]["playNavigationEndpoint"]["watchEndpoint"]["playlistSetVideoId"]
//                                     .as_str()
//                                     .unwrap_or("")
//                                     .to_string(),
//                                 title,
//                                 duration,
//                             });
//                         }
//                     }
//                 }
//             }
//         }
//     }
//     Ok(songs)
// }
//
pub fn parse_search_songs(data: &str) -> YResult<Vec<Song>> {
    let contents = gjson::get(
        data,
        "contents.tabbedSearchResultsRenderer.tabs.0.tabRenderer.content.sectionListRenderer.contents",
    );
    let mut songs = Vec::new();
    contents.each(|_, section| {
        let items = section.get("musicShelfRenderer.contents");
        if !items.exists() {
            return true;
        }
        items.each(|_, item| {
            let renderer = item.get("musicResponsiveListItemRenderer");
            if !renderer.exists() {
                return true;
            }

            let title_v = renderer.get("flexColumns.0.musicResponsiveListItemFlexColumnRenderer.text.runs.0.text");
            let video_id_v = renderer.get("overlay.musicItemThumbnailOverlayRenderer.content.musicPlayButtonRenderer.playNavigationEndpoint.watchEndpoint.videoId");
            if !video_id_v.str().is_empty() {
                let set_video_id = renderer.get("overlay.musicItemThumbnailOverlayRenderer.content.musicPlayButtonRenderer.playNavigationEndpoint.watchEndpoint.playlistSetVideoId");

                let mut duration = String::new();
                let runs = renderer.get("flexColumns.1.musicResponsiveListItemFlexColumnRenderer.text.runs");
                if runs.exists() {
                    let mut last_text = String::new();
                    runs.each(|_, run| {
                        last_text = run.get("text").str().to_string();
                        true
                    });
                    if last_text.contains(':') {
                        duration = last_text.trim().to_string();
                    }
                }

                songs.push(Song {
                    video_id: video_id_v.str().to_string(),
                    set_video_id: set_video_id.str().to_string(),
                    title: if title_v.str().is_empty() { "Unknown".to_string() } else { title_v.str().to_string() },
                    duration,
                });
            }
            true
        });
        true
    });
    Ok(songs)
}

// pub fn parse_params(data: Value) -> YResult<String> {
//     let params = data
//         .pointer("/contents/singleColumnMusicWatchNextResultsRenderer/tabbedRenderer/watchNextTabbedResultsRenderer/tabs/0/tabRenderer/content/musicQueueRenderer/content/playlistPanelRenderer/contents/1/automixPreviewVideoRenderer/content/automixPlaylistVideoRenderer/navigationEndpoint/watchPlaylistEndpoint/params")
//         .and_then(|v| v.as_str())
//         .map(|s| s.to_string());
//     params.ok_or(YError::InvalidResponse("Get Params".to_string()))
// }

pub fn parse_params(data: &str) -> YResult<String> {
    let path = "contents.singleColumnMusicWatchNextResultsRenderer.tabbedRenderer.watchNextTabbedResultsRenderer.tabs.0.tabRenderer.content.musicQueueRenderer.content.playlistPanelRenderer.contents.1.automixPreviewVideoRenderer.content.automixPlaylistVideoRenderer.navigationEndpoint.watchPlaylistEndpoint.params";
    let val = gjson::get(data, path);
    if val.exists() {
        Ok(val.str().to_string())
    } else {
        Err(YError::InvalidResponse("Get Params".to_string()))
    }
}

// pub fn parse_related_songs(data: Value) -> YResult<Vec<Song>> {
//     let mut songs = Vec::new();
//     let contents = data.pointer("/contents/singleColumnMusicWatchNextResultsRenderer/tabbedRenderer/watchNextTabbedResultsRenderer/tabs/0/tabRenderer/content/musicQueueRenderer/content/playlistPanelRenderer/contents").and_then(|c|c.as_array()).ok_or(YError::InvalidResponse("Get related songs".to_string()))?;
//     for item in contents {
//         if let Some(video) = item.get("playlistPanelVideoRenderer") {
//             let video_id = video.get("videoId").and_then(|v| v.as_str());
//             let title = video.pointer("/title/runs/0/text").and_then(|v| v.as_str());
//             let duration = video
//                 .pointer("/lengthText/runs/0/text")
//                 .and_then(|v| v.as_str());
//
//             if let (Some(vid), Some(t), Some(duration)) = (video_id, title, duration) {
//                 songs.push(Song {
//                     video_id: vid.to_string(),
//                     set_video_id: "".to_string(),
//                     title: t.to_string(),
//                     duration: duration.to_string(),
//                 });
//             }
//         }
//     }
//     Ok(songs)
// }

pub fn parse_related_songs(data: &str) -> YResult<Vec<Song>> {
    let json = gjson::get(
        data,
        "contents.singleColumnMusicWatchNextResultsRenderer.tabbedRenderer.watchNextTabbedResultsRenderer.tabs.0.tabRenderer.content.musicQueueRenderer.content.playlistPanelRenderer.contents",
    );

    let mut songs = Vec::new();
    json.each(|_, item| {
        if songs.len() >= 30 {
            return false;
        }
        let video = item.get("playlistPanelVideoRenderer");
        if video.exists() {
            let video_id = video.get("videoId");
            let title = video.get("title.runs.0.text");
            let duration = video.get("lengthText.runs.0.text");
            if !video_id.str().is_empty() {
                songs.push(Song {
                    video_id: video_id.str().to_string(),
                    set_video_id: String::new(),
                    title: if title.str().is_empty() {
                        "Unknown".to_string()
                    } else {
                        title.str().to_string()
                    },
                    duration: duration.str().to_string(),
                });
            }
        }
        true
    });
    Ok(songs)
}
