use crate::app::App;
use api::YClient;
use crossterm::event::{KeyCode, KeyEvent};
use data::{AppPage, FocusArea, MpvEvent, PlayMode, PlayerStatus};
use error::log_to_file;
use player::Player;
use state::AppState;
use std::sync::Arc;

pub fn handle_mpv_event(app: &mut App, player: &mut Player, state: &mut AppState, event: MpvEvent) {
    match event {
        MpvEvent::ListChange(list) => {
            app.mpv_list = list;
            if !app.mpv_list.is_empty() && app.playing_song.is_some() {
                player.status = PlayerStatus::Loading;
            }
        }
        MpvEvent::StartPlaying(video_id) => {
            for song in &app.songs {
                if song.video_id == video_id {
                    app.playing_song = Some(song.clone());
                    app.time_pos = Some(0.0);
                }
            }
            player.status = PlayerStatus::Playing;
        }
        MpvEvent::VolumeChange(vol) => {
            state.player_state.volume = vol;
            if let Err(e) = state.save() {
                log_to_file(&e);
            }
        }
        MpvEvent::TimePos(pos) => {
            app.time_pos = Some(pos);
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
    if key_event.code == KeyCode::Tab {
        handle_page_event(app);
    }
    if !app.is_insert || app.page == AppPage::Library {
        match key_event.code {
            KeyCode::Char('q') => {
                app.is_exit = true;
            }
            KeyCode::Char('3') => {
                app.focus_area = FocusArea::Queue;
                if app.songs_liststate.selected().is_none() && !app.songs.is_empty() {
                    app.songs_liststate.select(Some(0));
                }
            }
            KeyCode::Char('c') => {
                if let Err(e) = player.clear_queue().await {
                    log_to_file(&e);
                } else {
                    app.playing_song = None;
                    app.songs = Vec::new();
                    app.playing_playlist_id = None;
                    app.notify(data::NotifyType::Success, String::from("Cleared Queue"));
                }
            }
            _ => {}
        }
        if app.focus_area == FocusArea::Queue {
            handle_queue_event(key_event, app, player).await;
        }
        handle_player_event(key_event, app, player, state).await
    }
    if app.focus_area == FocusArea::Albums
        || app.focus_area == FocusArea::Playlists
        || app.focus_area == FocusArea::Queue
        || (app.focus_area == FocusArea::SearchAlbums && !app.is_insert)
        || (app.focus_area == FocusArea::SearchSongs && !app.is_insert)
    {
        handle_lists_event(key_event, app);
    }
    match app.page {
        AppPage::Library => match key_event.code {
            KeyCode::Char('1') => {
                app.focus_area = FocusArea::Albums;
            }
            KeyCode::Char('2') => {
                app.focus_area = FocusArea::Playlists;
            }
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
                                app.focus_area = FocusArea::Queue;
                                app.playing_playlist_id = Some(browse_id.clone());
                                app.playing_song = None;
                                if let Err(e) = player.load_playlist(&app.songs).await {
                                    log_to_file(&e);
                                }
                            }
                        } else {
                            log_to_file("Fetching songs Error");
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
                            if let Err(e) = client.remove_saved_list(&id).await {
                                log_to_file(&e);
                            } else {
                                app.albums.remove(i);
                                app.notify(
                                    data::NotifyType::Success,
                                    String::from("Removed album from Library"),
                                );
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
                        if i == 0 || i == app.playlists.len() - 1 {
                            app.notify(
                                data::NotifyType::Error,
                                String::from("Can not remove this playlist"),
                            );
                        } else if let Some(playlist) = app.playlists.get(i) {
                            let id = &playlist.playlist_id;
                            let result = if playlist.is_custom {
                                client.remove_saved_cus_list(id).await
                            } else {
                                client.remove_saved_list(id).await
                            };
                            match result {
                                Ok(_) => {
                                    app.playlists.remove(i);
                                    app.notify(
                                        data::NotifyType::Success,
                                        String::from("Removed playlist from Library"),
                                    );
                                }
                                Err(e) => log_to_file(&e),
                            }
                        }
                    }
                }
                _ => {}
            },
            _ => {}
        },
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
                                            app.notify(
                                                data::NotifyType::Success,
                                                String::from("Saved album to Library"),
                                            );
                                        }
                                    }
                                }
                            }
                        } else if app.focus_area == FocusArea::SearchSongs {
                            if let Some(i) = app.search_songs_liststate.selected() {
                                if let Some(song) = app.search_songs.get(i) {
                                    if let Err(e) = player.append_to_queue(&song.video_id).await {
                                        log_to_file(&e);
                                    } else {
                                        let new_song = song.clone();
                                        app.songs.push(new_song);
                                        app.notify(
                                            data::NotifyType::Success,
                                            String::from("Added song to Queue"),
                                        );
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
                                            client.remove_saved_list(&selected.playlist_id).await
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
                                                app.notify(
                                                    data::NotifyType::Success,
                                                    String::from("Removed Album from Library"),
                                                );
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
                                        app.focus_area = FocusArea::Queue;
                                        app.playing_playlist_id = Some(browse_id.clone());
                                        app.playing_song = None;
                                        if let Err(e) = player.load_playlist(&app.songs).await {
                                            log_to_file(&e);
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
                                        app.focus_area = FocusArea::Queue;
                                        app.playing_playlist_id = None;
                                        app.playing_song = None;
                                        if let Err(e) = player.load_playlist(&app.songs).await {
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
}

fn handle_lists_event(key_event: KeyEvent, app: &mut App) {
    match key_event.code {
        KeyCode::Down | KeyCode::Char('j') => app.next(),
        KeyCode::Up | KeyCode::Char('k') => app.previous(),
        _ => {}
    }
}
async fn handle_queue_event(key_event: KeyEvent, app: &mut App, player: &mut Player) {
    match key_event.code {
        KeyCode::Char('d') => {
            if let Some(i) = app.songs_liststate.selected() {
                if player.play_mode == PlayMode::DefaultMode {
                    if let Err(e) = player.remove_from_queue(i).await {
                        log_to_file(&e);
                    } else {
                        app.songs.remove(i);
                        app.notify(
                            data::NotifyType::Success,
                            String::from("Removed song from Queue"),
                        );
                    }
                } else {
                    let video_id = &app.songs[i].video_id;
                    if let Some(idx_mpv) = app.get_mpv_idx(video_id) {
                        if let Err(e) = player.remove_from_queue(idx_mpv).await {
                            log_to_file(&e);
                        } else {
                            app.songs.remove(i);
                            app.notify(
                                data::NotifyType::Success,
                                String::from("Removed song from Queue"),
                            );
                        }
                    }
                }
                if app.songs.is_empty() {
                    player.status = PlayerStatus::Idle;
                    app.playing_song = None;
                }
            }
        }
        KeyCode::Enter => {
            if let Some(i) = app.songs_liststate.selected() {
                if player.play_mode == PlayMode::DefaultMode {
                    if let Err(e) = player.play_at_idx(&i).await {
                        log_to_file(&e);
                    }
                } else {
                    let target_id = &app.songs[i].video_id;
                    if let Some(pos) = app.get_mpv_idx(target_id) {
                        if let Err(e) = player.play_at_idx(&pos).await {
                            log_to_file(&e);
                        } else {
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
fn handle_page_event(app: &mut App) {
    match app.page {
        AppPage::Library => {
            app.page = AppPage::Search;
            if app.songs.is_empty() && app.search_songs.is_empty() {
                app.is_insert = true;
            } else if app.focus_area != FocusArea::Queue {
                app.focus_area = FocusArea::SearchAlbums;
            }
        }
        AppPage::Search => {
            // app.is_insert = false;
            app.page = AppPage::Library;
            if app.focus_area != FocusArea::Queue {
                app.focus_area = FocusArea::Albums;
            }
        }
    }
}
async fn handle_player_event(
    key_event: KeyEvent,
    app: &mut App,
    player: &mut Player,
    state: &mut AppState,
) {
    match key_event.code {
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
        KeyCode::Left => {
            if let Err(e) = player.backward().await {
                log_to_file(&e);
            }
        }
        KeyCode::Right => {
            if let Err(e) = player.forward().await {
                log_to_file(&e);
            }
        }
        _ => {}
    }
}
