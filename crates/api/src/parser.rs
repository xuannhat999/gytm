use data::app::{Playlist, Song};
use error::{YError, YResult};

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

fn extract_artist(runs: &gjson::Value) -> String {
    let mut parts: Vec<String> = Vec::new();
    runs.each(|_, run| {
        let t = run.get("text").str().trim().to_string();
        if t == "•" || t == "|" {
            return false;
        }
        parts.push(run.get("text").str().to_string());
        true
    });
    parts.join("")
}

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
    let album_artist_runs = gjson::get(data, "contents.twoColumnBrowseResultsRenderer.tabs.0.tabRenderer.content.sectionListRenderer.contents.0.musicResponsiveHeaderRenderer.straplineTextOne.runs");
    let album_artist = if album_artist_runs.exists() { extract_artist(&album_artist_runs) } else { String::new() };

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
            let runs = r.get("flexColumns.1.musicResponsiveListItemFlexColumnRenderer.text.runs");
            let mut artist = if runs.exists() { extract_artist(&runs) } else { String::new() };
            if artist.is_empty() {
                artist = album_artist.clone();
            }
            songs.push(Song {
                video_id: video_id.str().to_string(),
                set_video_id: set_video_id.str().to_string(),
                title: if title.str().is_empty() { "Unknown".to_string() } else { title.str().to_string() },
                artist,
                duration: duration.str().to_string(),
            });
        }
        true
    });
    Ok(songs)
}

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
                let mut artist = String::new();
                if runs.exists() {
                    let mut last_text = String::new();
                    runs.each(|_, run| {
                        last_text = run.get("text").str().to_string();
                        true
                    });
                    if last_text.contains(':') {
                        duration = last_text.trim().to_string();
                    }
                    artist = extract_artist(&runs);
                }

                songs.push(Song {
                    video_id: video_id_v.str().to_string(),
                    set_video_id: set_video_id.str().to_string(),
                    title: if title_v.str().is_empty() { "Unknown".to_string() } else { title_v.str().to_string() },
                    artist,
                    duration,
                });
            }
            true
        });
        true
    });
    Ok(songs)
}

pub fn parse_params(data: &str) -> YResult<String> {
    let path = "contents.singleColumnMusicWatchNextResultsRenderer.tabbedRenderer.watchNextTabbedResultsRenderer.tabs.0.tabRenderer.content.musicQueueRenderer.content.playlistPanelRenderer.contents.1.automixPreviewVideoRenderer.content.automixPlaylistVideoRenderer.navigationEndpoint.watchPlaylistEndpoint.params";
    let val = gjson::get(data, path);
    if val.exists() {
        Ok(val.str().to_string())
    } else {
        Err(YError::InvalidResponse("Get Params".to_string()))
    }
}

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
            let runs = video.get("longBylineText.runs");
            let artist = if runs.exists() { extract_artist(&runs) } else { String::new() };
            if !video_id.str().is_empty() {
                songs.push(Song {
                    video_id: video_id.str().to_string(),
                    set_video_id: String::new(),
                    title: if title.str().is_empty() {
                        "Unknown".to_string()
                    } else {
                        title.str().to_string()
                    },
                    artist,
                    duration: duration.str().to_string(),
                });
            }
        }
        true
    });
    Ok(songs)
}
