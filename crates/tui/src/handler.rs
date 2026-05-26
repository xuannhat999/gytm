use std::os::linux::raw::stat;
use std::sync::Arc;

use crate::app::App;
use api::YClient;
use crossterm::event::{KeyCode, KeyEvent};
use data::FocusArea;
use data::{MpvEvent, PlayMode, PlayerStatus};
use error::log_to_file;
use player::Player;
use state::AppState;

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
    match key_event.code {
        KeyCode::Char('q') => app.is_exit = true,
        KeyCode::Tab => app.toggle_focus(),
        KeyCode::Char('1') => {
            app.focus_area = FocusArea::Albums;
            app.album_list_state
                .select(app.album_list_state.selected().or(Some(0)));
        }
        KeyCode::Char('2') => {
            app.focus_area = FocusArea::Playlists;
            app.playlist_list_state
                .select(app.playlist_list_state.selected().or(Some(0)));
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
        KeyCode::Char('p') => {
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
                    app.album_list_state
                        .selected()
                        .map(|i| &app.albums[i].browse_id)
                } else {
                    app.playlist_list_state
                        .selected()
                        .map(|i| &app.playlists[i].browse_id)
                };
                if let Some(browse_id) = selection {
                    if let Ok(songs) = client.get_songs(browse_id).await {
                        if !songs.is_empty() {
                            app.songs = songs;
                            app.songs_list_state.select(Some(0));
                            app.focus_area = FocusArea::SongList;
                            app.viewing_playlist_id = Some(browse_id.clone());
                            if key_event.code == KeyCode::Enter {
                                app.playing_playlist_id = app.viewing_playlist_id.clone();
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
                                if is_album {
                                    app.playlist_list_state.select(None);
                                } else {
                                    app.album_list_state.select(None);
                                }
                            }
                        }
                    } else {
                        log_to_file("Fetching songs Error");
                    }
                }
            }

            FocusArea::SongList => {
                if let Some(i) = app.songs_list_state.selected() {
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
                            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                            if let Err(e) = player.shuffle().await {
                                log_to_file(&e);
                            }
                        }
                    }
                }
            }
        },
        _ => match app.focus_area {
            FocusArea::Albums => handle_lists_event(key_event, app),
            FocusArea::Playlists => handle_playlists_events(key_event, app),
            FocusArea::SongList => handle_songs_events(key_event, app),
        },
    }
}

// ALBUM EVENT
fn handle_lists_event(key_event: KeyEvent, app: &mut App) {
    match key_event.code {
        KeyCode::Down | KeyCode::Char('j') => app.next(),
        KeyCode::Up | KeyCode::Char('k') => app.previous(),
        _ => {}
    }
}

// PLAYLIST EVENT
fn handle_playlists_events(key_event: KeyEvent, app: &mut App) {
    match key_event.code {
        KeyCode::Down | KeyCode::Char('j') => app.next(),
        KeyCode::Up | KeyCode::Char('k') => app.previous(),
        _ => {}
    }
}

// SONG LIST EVENT
fn handle_songs_events(key_event: KeyEvent, app: &mut App) {
    match key_event.code {
        KeyCode::Down | KeyCode::Char('j') => app.next(),
        KeyCode::Up | KeyCode::Char('k') => app.previous(),
        _ => {}
    }
}
