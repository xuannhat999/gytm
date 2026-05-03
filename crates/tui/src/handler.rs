use std::sync::Arc;

use crate::app::{App, FocusArea};
use api::YClient;
use crossterm::event::{KeyCode, KeyEvent};
use player::PlayerState;

pub async fn handle_key_events(key_event: KeyEvent, app: &mut App, client: Arc<YClient>) {
    match key_event.code {
        KeyCode::Char('q') => app.is_exit = true,
        KeyCode::Tab => app.toggle_focus(),
        KeyCode::Char('1') => app.focus_area = FocusArea::Albums,
        KeyCode::Char('2') => app.focus_area = FocusArea::Playlists,
        KeyCode::Char('3') => app.focus_area = FocusArea::SongList,
        KeyCode::Char(' ') => {
            if !app.player.current_process.is_none() && !app.player.current_song_idx.is_none() {
                if app.player.state == PlayerState::Playing {
                    app.player.toggle_pause();
                } else {
                    app.player.resume();
                }
            }
        }
        KeyCode::Enter => match app.focus_area {
            FocusArea::Albums => {
                if let Some(i) = app.album_list_state.selected() {
                    let browse_id = app.albums[i].browse_id.clone();
                    app.is_loading = true;
                    if let Ok(data_songs) = client.get_playlist_songs(&browse_id).await {
                        let songs = data::extract_songs_from_album(&data_songs);
                        app.songs = songs;
                        app.player.current_song_idx = Some(0);
                        if !app.songs.is_empty() {
                            app.songs_list_state.select(Some(0));
                        } else {
                            app.songs_list_state.select(None);
                        }
                        app.player.start_playlist(&app.songs, 0);
                        app.focus_area = FocusArea::SongList;
                    }
                    app.is_loading = false;
                }
            }
            FocusArea::SongList => {
                if let Some(i) = app.songs_list_state.selected() {
                    app.player.current_song_idx = Some(i);
                    if app.player.current_process.is_none() {
                        app.player.start_playlist(&app.songs, i);
                    } else {
                        app.player.jump_to_index(i);
                    }
                }
            }
            _ => {}
        },
        _ => match app.focus_area {
            FocusArea::Albums => handle_lists_event(key_event, app),
            FocusArea::Playlists => handle_playlists_events(key_event, app),
            FocusArea::SongList => handle_songs_events(key_event, app),
        },
    }
}

fn handle_lists_event(key_event: KeyEvent, app: &mut App) {
    match key_event.code {
        KeyCode::Down | KeyCode::Char('j') => app.next(),
        KeyCode::Up | KeyCode::Char('k') => app.previous(),
        _ => {}
    }
}

fn handle_playlists_events(key_event: KeyEvent, app: &mut App) {
    match key_event.code {
        KeyCode::Down | KeyCode::Char('j') => app.next(),
        KeyCode::Up | KeyCode::Char('k') => app.previous(),
        KeyCode::Enter => { /* Nạp playlist */ }
        _ => {}
    }
}

fn handle_songs_events(key_event: KeyEvent, app: &mut App) {
    match key_event.code {
        KeyCode::Down | KeyCode::Char('j') => app.next(),
        KeyCode::Up | KeyCode::Char('k') => app.previous(),
        _ => {}
    }
}
