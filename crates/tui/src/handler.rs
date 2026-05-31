use crate::app::App;
use api::YClient;
use crossterm::event::{KeyCode, KeyEvent};
use data::{AppPage, FocusArea};
use data::{MpvEvent, PlayMode, PlayerStatus};
use error::log_to_file;
use player::Player;
use state::AppState;
use std::sync::Arc;

pub fn handle_mpv_event(app: &mut App, player: &mut Player, state: &mut AppState, event: MpvEvent) {
    match event {
        MpvEvent::ListChange(list) => {
            app.mpv_list = list;
            if !app.mpv_list.is_empty() {
                player.status = PlayerStatus::Loading;
            }
        }
        MpvEvent::StartPlaying(song) => {
            app.playing_song = Some(song);
            player.status = PlayerStatus::Playing;
        }
        MpvEvent::VolumeChange(vol) => {
            state.player_state.volume = vol;
            if let Err(e) = state.save() {
                log_to_file(&e);
            }
        }
    }
}
pub async fn handle_key_events(
    key_event: KeyEvent,
    app: &mut App,
    client: Arc<YClient>,
    player: &mut Player,
    state: &mut AppState,
) {
    match app.page {
        AppPage::Library => {
            match key_event.code {
                KeyCode::Char('s') => {
                    app.page = AppPage::Search;
                    if app.search_albums.is_empty() {
                        app.is_insert = true;
                    } else {
                        app.focus_area = FocusArea::SearchAlbums;
                    }
                }
                KeyCode::Char('1') => {
                    app.focus_area = FocusArea::Albums;
                }
                KeyCode::Char('2') => {
                    app.focus_area = FocusArea::Playlists;
                }
                KeyCode::Char('3') => app.focus_area = FocusArea::SongList,
                KeyCode::Char(' ') if app.playing_song.is_some() => {
                    if let Err(e) = player.toggle_pause().await {
                        log_to_file(&e);
                    }
                }
                KeyCode::Char('m') => {
                    if let Err(e) = player.toggle_playmode().await {
                        log_to_file(&e);
                    } else {
                        state.player_state.play_mode = player.play_mode.clone();
                        if let Err(e) = state.save() {
                            log_to_file(&e);
                        }
                    }
                }
                KeyCode::Char('n') => {
                    if !app.songs.is_empty()
                        && let Err(e) = player.next().await
                    {
                        log_to_file(&e);
                    }
                }
                KeyCode::Char('b') => {
                    if !app.songs.is_empty()
                        && let Err(e) = player.prev().await
                    {
                        log_to_file(&e);
                    }
                }
                KeyCode::Char('-') => {
                    if let Err(e) = player.decrease_volume().await {
                        log_to_file(&e);
                    }
                }
                KeyCode::Char('+') => {
                    if let Err(e) = player.increase_volume().await {
                        log_to_file(&e);
                    }
                }
                // --- ENTER
                KeyCode::Enter => match app.focus_area {
                    FocusArea::Albums | FocusArea::Playlists => {
                        let is_album = app.focus_area == FocusArea::Albums;
                        let selection = if is_album {
                            app.albums_liststate
                                .selected()
                                .map(|i| &app.albums[i].browse_id)
                        } else {
                            app.playlists_liststate
                                .selected()
                                .map(|i| &app.playlists[i].browse_id)
                        };
                        if let Some(browse_id) = selection {
                            if let Ok(songs) = client.get_songs(browse_id).await {
                                if !songs.is_empty() {
                                    app.songs = songs;
                                    app.songs_liststate.select(Some(0));
                                    app.focus_area = FocusArea::SongList;
                                    app.playing_playlist_id = Some(browse_id.clone());
                                    if let Err(e) = player.load_playlist(&app.songs).await {
                                        log_to_file(&e);
                                    }
                                    if player.play_mode == PlayMode::ShuffleMode {
                                        tokio::time::sleep(tokio::time::Duration::from_millis(50))
                                            .await;
                                        if let Err(e) = player.shuffle().await {
                                            log_to_file(&e);
                                        }
                                    }
                                }
                            } else {
                                log_to_file("Fetching songs Error");
                            }
                        }
                    }
                    FocusArea::SongList => {
                        if let Some(i) = app.songs_liststate.selected() {
                            let target_id = &app.songs[i].video_id;
                            if let Some(pos) = app.get_mpv_idx(target_id) {
                                if let Err(e) = player.play_at_idx(&pos).await {
                                    log_to_file(&e);
                                }
                            } else {
                                log_to_file("Failed to get mpv index");
                            }
                        }
                    }
                    _ => {}
                },
                KeyCode::Char('d') => match app.focus_area {
                    FocusArea::Albums => {
                        if let Some(i) = app.albums_liststate.selected() {
                            let playlist_id = app.albums.get(i).map(|a| a.playlist_id.clone());
                            if let Some(id) = playlist_id {
                                if let Err(e) = client.remove_from_lib(&id).await {
                                    log_to_file(&e);
                                } else {
                                    app.albums.remove(i);
                                    if let Some(pos) =
                                        app.search_albums.iter().position(|a| id == a.playlist_id)
                                    {
                                        app.search_albums[pos].is_saved = false;
                                    }
                                }
                            }
                        }
                    }
                    FocusArea::Playlists => {
                        if let Some(i) = app.playlists_liststate.selected() {
                            let playlist_id = app.playlists.get(i).map(|a| a.playlist_id.clone());
                            if let Some(id) = playlist_id {
                                if let Err(e) = client.remove_from_lib(&id).await {
                                    log_to_file(&e);
                                } else {
                                    app.playlists.remove(i);
                                }
                            }
                        }
                    }
                    _ => {}
                },
                _ => {}
            }
        }
        AppPage::Search => {
            if app.is_insert {
                match key_event.code {
                    KeyCode::Char(c) => {
                        app.search_query.push(c);
                    }
                    KeyCode::Backspace => {
                        app.search_query.pop();
                    }
                    KeyCode::Enter => {
                        app.is_insert = false;
                        let (albums_res, songs_res) = tokio::join!(
                            client.get_search_albums(&app.search_query),
                            client.get_search_songs(&app.search_query)
                        );

                        match albums_res {
                            Ok(albums) => {
                                app.search_albums = albums;
                                app.search_albums_liststate.select(Some(0));
                                app.focus_area = FocusArea::SearchAlbums;
                            }
                            Err(e) => {
                                log_to_file(&e);
                            }
                        }

                        match songs_res {
                            Ok(songs) => {
                                app.search_songs = songs;
                                app.search_songs_liststate.select(Some(0));
                            }
                            Err(e) => {
                                log_to_file(&e);
                            }
                        }
                    }
                    KeyCode::Esc => {
                        app.is_insert = false;
                    }
                    _ => {}
                }
            } else {
                match key_event.code {
                    KeyCode::Char('1') => app.focus_area = FocusArea::SearchAlbums,
                    KeyCode::Char('2') => app.focus_area = FocusArea::SearchSongs,
                    KeyCode::Char('s') => {
                        app.is_insert = true;
                    }
                    KeyCode::Esc => {
                        app.page = AppPage::Library;
                        app.focus_area = FocusArea::Albums;
                    }
                    KeyCode::Char('a') => {
                        if app.focus_area == FocusArea::SearchAlbums {
                            if let Some(i) = app.search_albums_liststate.selected() {
                                if let Some(selected) = app.search_albums.get_mut(i) {
                                    if !selected.is_saved {
                                        if let Err(e) =
                                            client.add_to_lib(&selected.playlist_id).await
                                        {
                                            log_to_file(&e);
                                        } else {
                                            selected.is_saved = true;
                                            let new_album = selected.clone();
                                            app.albums.push(new_album);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    KeyCode::Char('d') => {
                        if app.focus_area == FocusArea::SearchAlbums {
                            if let Some(i) = app.search_albums_liststate.selected() {
                                if let Some(selected) = app.search_albums.get_mut(i) {
                                    if selected.is_saved {
                                        if let Err(e) =
                                            client.remove_from_lib(&selected.playlist_id).await
                                        {
                                            log_to_file(&e);
                                        } else {
                                            selected.is_saved = false;
                                            if let Some(pos) = app
                                                .albums
                                                .iter()
                                                .position(|a| a.playlist_id == selected.playlist_id)
                                            {
                                                app.albums.remove(pos);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    KeyCode::Enter => {
                        if app.focus_area == FocusArea::SearchAlbums {
                            let selected = app
                                .search_albums_liststate
                                .selected()
                                .map(|i| &app.search_albums[i].browse_id);
                            if let Some(browse_id) = selected {
                                if let Ok(songs) = client.get_songs(browse_id).await {
                                    if !songs.is_empty() {
                                        app.songs = songs;
                                        app.songs_liststate.select(Some(0));
                                        app.page = AppPage::Library;
                                        app.focus_area = FocusArea::SongList;
                                        app.playing_playlist_id = Some(browse_id.clone());
                                        if let Err(e) = player.load_playlist(&app.songs).await {
                                            log_to_file(&e);
                                        }
                                        if player.play_mode == PlayMode::ShuffleMode {
                                            tokio::time::sleep(tokio::time::Duration::from_millis(
                                                50,
                                            ))
                                            .await;
                                            if let Err(e) = player.shuffle().await {
                                                log_to_file(&e);
                                            }
                                        }
                                    }
                                }
                            }
                        } else if app.focus_area == FocusArea::SearchSongs {
                            let selected = app
                                .search_songs_liststate
                                .selected()
                                .map(|i| &app.search_songs[i]);
                            if let Some(song) = selected {
                                let video_id = &song.video_id;
                                if let Ok(params) = client.get_params(video_id).await {
                                    if let Ok(mut related_songs) =
                                        client.get_related_songs(video_id, &params).await
                                    {
                                        related_songs.insert(0, song.clone());
                                        app.songs = related_songs;
                                        app.songs_liststate.select(Some(0));
                                        app.page = AppPage::Library;
                                        app.focus_area = FocusArea::SongList;
                                        app.playing_playlist_id = None;
                                        if let Err(e) = player.load_playlist(&app.songs).await {
                                            log_to_file(&e);
                                        }
                                        if player.play_mode == PlayMode::ShuffleMode {
                                            tokio::time::sleep(tokio::time::Duration::from_millis(
                                                50,
                                            ))
                                            .await;
                                            if let Err(e) = player.shuffle().await {
                                                log_to_file(&e);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    if key_event.code == KeyCode::Char('q') && !app.is_insert {
        app.is_exit = true;
    }
    if app.focus_area == FocusArea::Albums
        || app.focus_area == FocusArea::Playlists
        || app.focus_area == FocusArea::SongList
        || (app.focus_area == FocusArea::SearchAlbums && !app.is_insert)
        || (app.focus_area == FocusArea::SearchSongs && !app.is_insert)
    {
        handle_lists_event(key_event, app);
    }
}

fn handle_lists_event(key_event: KeyEvent, app: &mut App) {
    match key_event.code {
        KeyCode::Down | KeyCode::Char('j') => app.next(),
        KeyCode::Up | KeyCode::Char('k') => app.previous(),
        _ => {}
    }
}
