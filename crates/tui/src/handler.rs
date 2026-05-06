use std::sync::Arc;

use crate::app::{App, FocusArea};
use api::YClient;
use crossterm::event::{KeyCode, KeyEvent};
use data::Song;
use player::{MpvEvent, PlayMode, Player, PlayerState, log_to_file};
use serde_json::Value;

pub fn handle_mpv_event(app: &mut App, player: &mut Player, event: MpvEvent) {
    match event {
        MpvEvent::ListChange(list) => {
            app.mpv_list = list;
            if !app.mpv_list.is_empty() {
                player.state = PlayerState::Loading;
            }
        }
        MpvEvent::StartPlaying(song) => {
            app.playing_song = Some(song);
            player.state = PlayerState::Playing;
        }
        _ => {}
    }
}
pub async fn handle_key_events(
    key_event: KeyEvent,
    app: &mut App,
    client: Arc<YClient>,
    player: &mut Player,
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
        KeyCode::Char(' ') => {
            if app.playing_song.is_some() {
                player.toggle_pause().await;
            }
        }
        KeyCode::Char('m') => {
            player.toggle_playmode().await;
        }

        // --- NEXT / PREV ---
        KeyCode::Char('n') => {
            player.next().await;
        }
        KeyCode::Char('p') => {
            player.prev().await;
        }

        // --- ENTER / L
        KeyCode::Enter | KeyCode::Char('l') => match app.focus_area {
            FocusArea::Albums | FocusArea::Playlists => {
                let is_album = app.focus_area == FocusArea::Albums;

                let selection = if is_album {
                    app.album_list_state.selected().map(|i| {
                        (
                            &app.albums[i].browse_id,
                            data::extract_songs_from_album as fn(&Value) -> Vec<Song>,
                        )
                    })
                } else {
                    app.playlist_list_state.selected().map(|i| {
                        (
                            &app.playlists[i].browse_id,
                            data::extract_songs_from_playlist as fn(&Value) -> Vec<Song>,
                        )
                    })
                };

                if let Some((browse_id, extractor)) = selection {
                    if let Ok(data_songs) = client.get_playlist_songs(browse_id).await {
                        let songs = extractor(&data_songs);
                        if !songs.is_empty() {
                            app.songs = songs;
                            app.songs_list_state.select(Some(0));
                            app.focus_area = FocusArea::SongList;
                            app.viewing_playlist = Some(browse_id.to_string());
                            if key_event.code == KeyCode::Enter {
                                app.playing_playlist = Some(browse_id.to_string());
                                player.load_playlist(&app.songs).await;
                                if player.play_mode == PlayMode::ShuffleMode {
                                    tokio::time::sleep(tokio::time::Duration::from_millis(50))
                                        .await;
                                    player.shuffle().await;
                                }
                                if is_album {
                                    app.playlist_list_state.select(None);
                                } else {
                                    app.album_list_state.select(None);
                                }
                            }
                        }
                    }
                }
                player::log_to_file(&format!("Viewing playlist: {:?}", &app.viewing_playlist));
                player::log_to_file(&format!("Playing playlist: {:?}", &app.playing_playlist));
            }
            FocusArea::SongList => {
                if let Some(i) = app.songs_list_state.selected() {
                    log_to_file("Played song");
                    let target_id = &app.songs[i].video_id;
                    if app.playing_playlist == app.viewing_playlist {
                        if let Some(pos) = app.get_mpv_idx(target_id) {
                            player.play_at_idx(&pos).await;
                        }
                    } else {
                        player.load_playlist(&app.songs).await;
                        tokio::time::sleep(tokio::time::Duration::from_millis(120)).await;
                        player.play_at_idx(&i).await;
                        app.playing_playlist = app.viewing_playlist.clone();
                        if player.play_mode == PlayMode::ShuffleMode {
                            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                            player.shuffle().await;
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
