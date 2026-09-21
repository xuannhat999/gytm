use super::{append_song_to_queue, load_list};
use crate::{app::App, notification::NotifyType};
use api::protocol::{ApiCmd, ApiLoadingKind, ApiResponse};
use data::app::{FocusArea, PopupState};
use error::{YError, log_to_file};
use player::Player;
use ratatui::widgets::ListState;
use state::Persist;

pub fn handle_api_response(app: &mut App, response: ApiResponse, player: &Player) {
    let mut skip_clear = false;
    match response {
        ApiResponse::CreatePlaylist(res) => match res {
            Ok(playlist) => {
                app.playlists.push(playlist);
                app.cus_playlists.push(app.playlists.len() - 1);
                app.popup_state = PopupState::None;
                app.noti
                    .notify(NotifyType::Success, "Created playlist".to_string());
            }
            Err(e) => {
                log_to_file(&e);
                app.noti
                    .notify(NotifyType::Error, format!("Failed to create playlist: {e}"));
            }
        },
        ApiResponse::SaveSong(res) => match res {
            Ok((song, playlist_id)) => {
                app.noti.notify(
                    NotifyType::Success,
                    format!("Saved '{}' to playlist", song.title),
                );
                if let Some(viewing_list) = &app.viewing_list
                    && viewing_list.playlist_id.eq(&playlist_id)
                {
                    app.songs.push(song.clone());
                }
                if let Some(playing_list) = &app.playing_playlist_id
                    && playing_list.eq(&playlist_id)
                {
                    if let Err(e) = append_song_to_queue(app, player, song) {
                        log_to_file(&e);
                    }
                }
            }
            Err(YError::AlreadyInPlaylist) => {
                app.noti
                    .notify(NotifyType::Error, "Song already in playlist".to_string());
            }
            Err(e) => {
                log_to_file(&e);
                app.noti
                    .notify(NotifyType::Error, format!("Failed to save song: {e}"));
            }
        },
        ApiResponse::Search {
            albums,
            songs,
            videos,
        } => {
            match albums {
                Ok(albums) => {
                    app.search_albums = albums;
                    if !app.search_albums.is_empty() {
                        app.focus_area = FocusArea::SearchAlbums;
                        app.search_albums_liststate.select(Some(0));
                    }
                }
                Err(e) => {
                    log_to_file(&e);
                }
            }
            match songs {
                Ok(songs) => {
                    app.search_songs = songs;
                    if !app.search_songs.is_empty() {
                        app.search_songs_liststate.select(Some(0));
                    }
                }
                Err(e) => {
                    log_to_file(&e);
                }
            }
            match videos {
                Ok(videos) => {
                    app.search_videos = videos;
                    if !app.search_videos.is_empty() {
                        app.search_videos_liststate.select(Some(0));
                    }
                }
                Err(e) => {
                    log_to_file(&e);
                }
            }
        }
        ApiResponse::LikeSong(res) => match res {
            Ok(song) => {
                app.noti
                    .notify(NotifyType::Success, format!("Liked '{}'", song.title));
                if let Some(viewing_list) = &app.viewing_list
                    && viewing_list.playlist_id.eq("LM")
                {
                    app.songs.push(song.clone());
                }
                if let Some(playing_list) = &app.playing_playlist_id
                    && playing_list.eq("LM")
                {
                    if let Err(e) = append_song_to_queue(app, player, song) {
                        log_to_file(&e);
                    }
                }
            }
            Err(e) => {
                log_to_file(&e);
                app.noti
                    .notify(NotifyType::Error, format!("Failed to like song: {e}"));
            }
        },
        ApiResponse::UnlikeSong((res, title)) => match res {
            Ok(_) => {
                app.noti
                    .notify(NotifyType::Success, format!("UnLiked '{}'", title));
            }
            Err(e) => {
                log_to_file(&e);
                app.noti.notify(
                    NotifyType::Error,
                    format!("Failed to unlike '{}'\nError: {}", title, e),
                );
            }
        },
        ApiResponse::UnsaveSong((res, title)) => match res {
            Ok(_) => {
                app.noti
                    .notify(NotifyType::Success, format!("Unsaved '{}'", title));
            }
            Err(e) => {
                log_to_file(&e);
                app.noti.notify(
                    NotifyType::Error,
                    format!("Failed to unsave '{}'\nError: {}", title, e),
                );
            }
        },
        ApiResponse::GetSongsToView { songs, playlist } => match songs {
            Ok(songs) => {
                app.songs = songs;
                app.viewing_list = Some(playlist);
                if !app.songs.is_empty() {
                    app.songs_liststate.select(Some(0));
                }
            }
            Err(e) => {
                log_to_file(&e);
                app.noti.notify(
                    NotifyType::Error,
                    format!("Failed to fetch songs\nError: {e}"),
                );
            }
        },
        ApiResponse::GetSongsToPlay { songs, playlist_id } => match songs {
            Ok(songs) => {
                load_list(app, player, songs, 0, Some(playlist_id)).ok();
            }
            Err(e) => {
                log_to_file(&e);
                app.noti.notify(
                    NotifyType::Error,
                    format!("Failed to fetch songs\nError: {e}"),
                );
            }
        },
        ApiResponse::UnsaveAlbum((res, list)) => match res {
            Ok(_) => {
                app.noti.notify(
                    NotifyType::Success,
                    format!("Unsaved album '{}'", list.title),
                );
            }
            Err(e) => {
                log_to_file(&e);
                app.noti.notify(
                    NotifyType::Error,
                    format!("Failed to unsave album '{}'\nError: {}", list.title, e),
                );
            }
        },
        ApiResponse::UnsaveCusPlaylist((res, title)) => match res {
            Ok(_) => {
                app.noti
                    .notify(NotifyType::Success, format!("Unsaved playlist '{}'", title));
                app.refresh_cus_playlist();
            }
            Err(e) => {
                log_to_file(&e);
                app.noti.notify(
                    NotifyType::Error,
                    format!("Failed to unsave playlist '{}'\nError: {}", title, e),
                );
            }
        },
        ApiResponse::SaveAlbum((res, album)) => match res {
            Ok(_) => {
                app.noti.notify(
                    NotifyType::Success,
                    format!("Saved album '{}'", album.title),
                );
                app.albums.push(album);
            }
            Err(e) => {
                log_to_file(&e);
                app.noti.notify(
                    NotifyType::Error,
                    format!("Failed to save album '{}'\nError: {}", album.title, e),
                );
            }
        },
        ApiResponse::GetRelatedSongsToPlay(songs) => match songs {
            Ok(related_songs) => {
                load_list(app, player, related_songs, 0, None).ok();
            }
            Err(e) => {
                log_to_file(&e);
                app.noti.notify(
                    NotifyType::Error,
                    format!("Failed to fetch related songs\nError: {e}"),
                );
            }
        },
        ApiResponse::FetchLibraryData(lib_data) => match lib_data {
            Ok((albums, playlists, cus_playlists)) => {
                app.albums = albums;
                app.playlists = playlists;
                app.cus_playlists = cus_playlists;
                if !app.albums.is_empty() {
                    app.albums_liststate.select(Some(0));
                }
                if !app.playlists.is_empty() {
                    app.playlists_liststate.select(Some(0));
                }
            }
            Err(e) => {
                log_to_file(&e);
                app.noti.notify(
                    NotifyType::Error,
                    format!("Failed to fetch Library data: {e}"),
                );
            }
        },

        ApiResponse::FetchAccountsList(res) => {
            match res {
                Ok((accounts, browser, profile, container)) => {
                    let mut liststate = ListState::default();
                    if !accounts.is_empty() {
                        liststate.select(Some(0));
                    }
                    app.popup_state = PopupState::SelectAccount {
                        accounts,
                        accounts_liststate: liststate,
                        browser,
                        profile,
                        container,
                    }
                }
                Err(e) => {
                    app.noti.notify(
                        NotifyType::Error,
                        format!("Failed to fetch account list: {e}"),
                    );
                    log_to_file(e);
                }
            }
            app.api_loading_kind = None;
        }
        ApiResponse::ReloadApiCLient(result) | ApiResponse::SetClient(result) => match result {
            Ok(client_state) => {
                app.noti.notify(
                    NotifyType::Success,
                    "Reloaded YTM client successfully".to_string(),
                );
                app.client_state = client_state;
                app.client_state.save().ok();
                app.api_cmd_tx.send(ApiCmd::FetchLibraryData).ok();
                app.api_loading_kind = Some(ApiLoadingKind::FetchLibraryData);
                skip_clear = true;
            }
            Err(e) => {
                log_to_file(&e);
                app.noti.notify(
                    NotifyType::Error,
                    format!("Failed to reload api client: {e}"),
                );
            }
        },
        ApiResponse::ToggleGuest(res) => match res {
            Ok(_) => {
                app.noti.notify(
                    NotifyType::Success,
                    "Toggled Guest mode, library feature is unavailable".to_string(),
                );
            }
            Err(e) => {
                log_to_file(e);
            }
        },
    }
    if !skip_clear {
        app.api_loading_kind = None;
    }
}
