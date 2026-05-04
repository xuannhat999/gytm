use std::sync::Arc;

use crate::app::{App, FocusArea};
use api::YClient;
use crossterm::event::{KeyCode, KeyEvent};
use data::Song;
use player::{MpvEvent, Player, PlayerState};
use serde_json::Value;

pub fn handle_mpv_event(app: &mut App, player: &mut Player, event: MpvEvent) {
    match event {
        MpvEvent::ListChange(data) => {
            if let Some(items) = data.as_array() {
                // if items.len() < 2 && !app.songs.is_empty() {}
            }
        }
        MpvEvent::StartPlaying(video_id) => {
            // println!("Recieved StartPlaying Event: {}", video_id);
            let idx = app.get_idx_from_id(video_id);
            app.song_idx = Some(idx);
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
        KeyCode::Char('q') => app.is_exit = true, // Q => QUIT APP
        KeyCode::Tab => app.toggle_focus(),       // TAB => SWITCH FOCUS TO OTHER FOCUS AREA
        KeyCode::Char('1') => {
            // 1 => FOCUS ALBUMS AREA
            app.focus_area = FocusArea::Albums;
            if app.album_list_state.selected().is_none() {
                app.album_list_state.select(Some(0));
            }
        }
        KeyCode::Char('2') => {
            // 2 => FOCUS PLAYLISTS AREA
            app.focus_area = FocusArea::Playlists;
            if app.playlist_list_state.selected().is_none() {
                app.playlist_list_state.select(Some(0));
            }
        } // 2 => FOCUS PLAYLISTS AREA
        KeyCode::Char('3') => app.focus_area = FocusArea::SongList, // 3 => FOCUS SONG LIST AREA

        KeyCode::Char(' ') => {
            // SPACE => TOGGLE PAUSE
            if !app.song_idx.is_none() {
                player.toggle_pause();
            } else {
                println!("No song idx");
            }
        }
        // KeyCode::Char('n') => match app.song_idx {
        //     // N => PLAY NEXT SONG IN LIST
        //     Some(idx) => {
        //         if idx < app.songs.len() - 1 {
        //             player.next();
        //         }
        //     }
        //     None => todo!(),
        // },
        // KeyCode::Char('p') => match app.song_idx {
        //     // P => PLAY PREVIOUS SONG IN LIST
        //     Some(idx) => {
        //         if idx > 0 {
        //             player.prev();
        //         }
        //     }
        //     None => todo!(),
        // },
        KeyCode::Enter => match app.focus_area {
            // ENTER AT ALBUMS / PLAYLISTS FOCUS AREA
            FocusArea::Albums | FocusArea::Playlists => {
                let target_info = match app.focus_area {
                    FocusArea::Albums => {
                        app.is_loading = true;
                        app.playlist_list_state.select(None);
                        app.album_list_state.selected().map(|i| {
                            (
                                app.albums[i].browse_id.clone(),
                                data::extract_songs_from_album as fn(&Value) -> Vec<Song>,
                            )
                        })
                    }
                    FocusArea::Playlists => {
                        app.is_loading = true;
                        app.album_list_state.select(None);
                        app.playlist_list_state.selected().map(|i| {
                            (
                                app.playlists[i].browse_id.clone(),
                                data::extract_songs_from_playlist as fn(&Value) -> Vec<Song>,
                            )
                        })
                    }
                    _ => None,
                };
                if let Some((browse_id, extractor)) = target_info {
                    app.is_loading = true;
                    if let Ok(data_songs) = client.get_playlist_songs(&browse_id).await {
                        let songs = extractor(&data_songs);
                        app.songs = songs;
                        if !app.songs.is_empty() {
                            player.load_song(&app.songs.first().unwrap().video_id, false);
                            app.songs_list_state.select(Some(0));
                            app.focus_area = FocusArea::SongList;
                        } else {
                            app.songs_list_state.select(None);
                        }
                    }
                }
                app.is_loading = false;
            } // ENTER AT SONG LIST FOCUS AREA
            FocusArea::SongList => {
                //     if let Some(i) = app.songs_list_state.selected() {
                //         app.song_idx = Some(i);
                //         if player.current_process.is_none() {
                //             player.start_playlist(&app.songs);
                //         } else {
                //             player.jump_to_index(i);
                //         }
                //     }
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
