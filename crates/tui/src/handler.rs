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
                KeyCode::Tab => app.toggle_focus(),
                KeyCode::Char('1') => {
                    app.focus_area = FocusArea::Albums;
                }
                KeyCode::Char('2') => {
                    app.focus_area = FocusArea::Playlists;
                    app.playlists_liststate
                        .select(app.playlists_liststate.selected().or(Some(0)));
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
                // --- ENTER / L
                KeyCode::Enter | KeyCode::Char('l') => match app.focus_area {
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
                                    app.viewing_playlist_id = Some(browse_id.clone());
                                    if key_event.code == KeyCode::Enter {
                                        app.playing_playlist_id = app.viewing_playlist_id.clone();
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
                            } else {
                                log_to_file("Fetching songs Error");
                            }
                        }
                    }
                    FocusArea::SongList => {
                        if let Some(i) = app.songs_liststate.selected() {
                            let target_id = &app.songs[i].video_id;
                            if app.playing_playlist_id == app.viewing_playlist_id {
                                if let Some(pos) = app.get_mpv_idx(target_id) {
                                    if let Err(e) = player.play_at_idx(&pos).await {
                                        log_to_file(&e);
                                    }
                                } else {
                                    log_to_file("Failed to get mpv index");
                                }
                            } else {
                                if let Err(e) = player.load_playlist(&app.songs).await {
                                    log_to_file(&e);
                                }
                                tokio::time::sleep(tokio::time::Duration::from_millis(120)).await;
                                if let Err(e) = player.play_at_idx(&i).await {
                                    log_to_file(&e);
                                }
                                app.playing_playlist_id = app.viewing_playlist_id.clone();
                                if player.play_mode == PlayMode::ShuffleMode {
                                    tokio::time::sleep(tokio::time::Duration::from_millis(50))
                                        .await;
                                    if let Err(e) = player.shuffle().await {
                                        log_to_file(&e);
                                    }
                                }
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
                        match client.get_search_albums(&app.search_query).await {
                            Ok(albums) => {
                                app.search_albums = albums;
                                app.search_albums_liststate.select(Some(0));
                                app.focus_area = FocusArea::SearchAlbums;
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
                                    app.viewing_playlist_id = Some(browse_id.clone());
                                    app.playing_playlist_id = Some(browse_id.clone());
                                    app.albums_liststate.select(None);
                                    app.playlists_liststate.select(None);
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
