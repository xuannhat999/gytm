use crate::app::{App, CreatePlaylistFocus, PopupState};
use api::YClient;
use crossterm::event::{KeyCode, KeyEvent};
use data::{AppPage, FocusArea, MpvEvent, PlayList, PlayListPrivacy, PlayMode, PlayerStatus, Song};
use error::{YError, log_to_file};
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
            for song in &app.queue {
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
    if app.focus_area == FocusArea::Albums
        || app.focus_area == FocusArea::Playlists
        || app.focus_area == FocusArea::Queue
        || (app.focus_area == FocusArea::SearchAlbums && !app.is_insert)
        || (app.focus_area == FocusArea::SearchSongs && !app.is_insert)
        || (app.focus_area == FocusArea::Songs)
    {
        handle_lists_event(key_event, app);
    }
    if !matches!(app.popup_state, PopupState::None) {
        handle_popup_event(key_event, app, client, player).await;
    } else {
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
                    if app.queue_liststate.selected().is_none() && !app.queue.is_empty() {
                        app.queue_liststate.select(Some(0));
                    }
                }
                KeyCode::Char('c') => {
                    if let Err(e) = player.clear_queue().await {
                        log_to_file(&e);
                    } else {
                        app.playing_song = None;
                        app.queue = Vec::new();
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

        match app.page {
            AppPage::Library => match key_event.code {
                KeyCode::Char('1') => {
                    app.focus_area = FocusArea::Albums;
                }
                KeyCode::Char('2') => {
                    app.focus_area = FocusArea::Playlists;
                }
                KeyCode::Char('4') => {
                    app.focus_area = FocusArea::Songs;
                }
                KeyCode::Char('l') => {
                    let list = if app.focus_area == FocusArea::Albums {
                        app.albums_liststate.selected().map(|i| &app.albums[i])
                    } else if app.focus_area == FocusArea::Playlists {
                        app.playlists_liststate
                            .selected()
                            .map(|i| &app.playlists[i])
                    } else {
                        None
                    };
                    if let Some(list) = list {
                        match client.get_songs(&list.browse_id).await {
                            Ok(result) => {
                                app.viewing_list = Some(list.clone());
                                app.songs = result;
                                app.focus_area = FocusArea::Songs;
                                if !app.songs.is_empty() {
                                    app.songs_liststate.select(Some(0));
                                }
                            }
                            Err(e) => {
                                log_to_file(&e);
                            }
                        };
                    }
                }
                KeyCode::Enter => match app.focus_area {
                    FocusArea::Albums | FocusArea::Playlists => {
                        let is_album = app.focus_area == FocusArea::Albums;
                        let selection = if is_album {
                            app.albums_liststate
                                .selected()
                                .map(|i| app.albums[i].clone())
                        } else {
                            app.playlists_liststate
                                .selected()
                                .map(|i| app.playlists[i].clone())
                        };
                        if let Some(list) = selection {
                            fetch_and_play_list(app, client, player, list).await;
                        }
                    }
                    FocusArea::Songs => {
                        if let Some(list) = &app.viewing_list {
                            let viewing_list_id = &list.playlist_id;
                            let is_dup = match &app.playing_playlist_id {
                                Some(playing) => {
                                    if playing == viewing_list_id {
                                        true
                                    } else {
                                        false
                                    }
                                }
                                None => false,
                            };
                            if !is_dup {
                                if let Some(i) = app.songs_liststate.selected() {
                                    app.playing_playlist_id = Some(list.playlist_id.clone());
                                    match player.load_playlist(&app.songs, i).await {
                                        Err(e) => {
                                            log_to_file(&e);
                                        }
                                        Ok(_) => {
                                            app.queue = app.songs.clone();
                                            app.focus_area = FocusArea::Queue;
                                            app.queue_liststate.select(Some(i));
                                        }
                                    }
                                }
                            } else {
                                app.notify(data::NotifyType::Error, String::from("Playlist/Album's already been playing, change song in Queue"));
                            }
                        }
                    }
                    _ => {}
                },
                KeyCode::Char('x') => match app.focus_area {
                    FocusArea::Albums => {
                        if let Some(i) = app.albums_liststate.selected() {
                            let playlist_id = app.albums.get(i).map(|a| a.playlist_id.clone());
                            if let Some(id) = playlist_id {
                                if let Err(e) = client.unsave_album_raw(&id).await {
                                    log_to_file(&e);
                                } else {
                                    app.albums.remove(i);
                                    app.notify(
                                        data::NotifyType::Success,
                                        String::from("Unsaved album from Library"),
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
                            if let Some(playlist) = app.playlists.get(i) {
                                if playlist.playlist_id == "LM" || playlist.playlist_id == "SE" {
                                    app.notify(
                                        data::NotifyType::Error,
                                        String::from("Can not remove this playlist"),
                                    );
                                } else {
                                    let id = &playlist.playlist_id;
                                    let res = if playlist.is_custom {
                                        client.unsave_cus_playlist_raw(id).await
                                    } else {
                                        client.unsave_album_raw(id).await
                                    };
                                    match res {
                                        Ok(_) => {
                                            app.playlists.remove(i);
                                            app.notify(
                                                data::NotifyType::Success,
                                                String::from("Unsaved playlist from Library"),
                                            );
                                        }
                                        Err(e) => log_to_file(&e),
                                    }
                                }
                            }
                        }
                    }
                    FocusArea::Songs => {
                        if let Some(list) = &app.viewing_list {
                            if list.is_custom {
                                if let Some(i) = app.songs_liststate.selected() {
                                    let song = &app.songs[i];
                                    let res = if list.playlist_id == "LM" {
                                        client.like_or_unlike_song(&song.video_id, false).await
                                    } else {
                                        client
                                            .unsave_to_playlist(
                                                &song.video_id,
                                                &list.playlist_id,
                                                &song.set_video_id,
                                            )
                                            .await
                                    };
                                    match res {
                                        Ok(_) => {
                                            app.songs.remove(i);
                                            app.notify(
                                                data::NotifyType::Success,
                                                String::from("Removed song from playlist"),
                                            );
                                        }
                                        Err(e) => {
                                            log_to_file(&e);
                                        }
                                    }
                                }
                            } else {
                                app.notify(
                                    data::NotifyType::Error,
                                    String::from("Unable to edit this Album/Playlistt"),
                                );
                            }
                        }
                    }

                    _ => {}
                },
                KeyCode::Char('a') => {
                    if app.focus_area == FocusArea::Songs {
                        if let Some(song) =
                            app.songs_liststate.selected().map(|i| app.songs[i].clone())
                        {
                            append_song_to_queue(app, player, song).await;
                        }
                    } else if app.focus_area == FocusArea::Playlists {
                        app.popup_state = PopupState::CreatePlaylist {
                            title: String::new(),
                            description: String::new(),
                            privacy: PlayListPrivacy::Private,
                            focused_field: CreatePlaylistFocus::Title,
                        };
                    }
                }

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
                        KeyCode::Char('x') => match app.focus_area {
                            FocusArea::SearchAlbums => {
                                if let Some(i) = app.search_albums_liststate.selected() {
                                    if let Some(selected) = app.search_albums.get_mut(i) {
                                        if !selected.is_saved {
                                            if let Err(e) =
                                                client.save_album_raw(&selected.playlist_id).await
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
                                        } else {
                                            if let Err(e) =
                                                client.unsave_album_raw(&selected.playlist_id).await
                                            {
                                                log_to_file(&e);
                                            } else {
                                                selected.is_saved = false;
                                                if let Some(pos) = app.albums.iter().position(|a| {
                                                    a.playlist_id == selected.playlist_id
                                                }) {
                                                    app.albums.remove(pos);
                                                    app.notify(
                                                        data::NotifyType::Success,
                                                        String::from("Unsaved album from Library"),
                                                    );
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            FocusArea::SearchSongs => {
                                if let Some(i) = app.search_songs_liststate.selected() {
                                    if let Some(song) = app.search_songs.get(i) {
                                        app.popup_state = PopupState::SaveSong {
                                            selected_save_song: song.clone(),
                                        };
                                        app.cus_playlists_liststate.select(Some(0));
                                    }
                                }
                            }
                            _ => {}
                        },
                        KeyCode::Char('a') => {
                            if let Some(song) = app
                                .search_songs_liststate
                                .selected()
                                .map(|i| app.search_songs[i].clone())
                            {
                                append_song_to_queue(app, player, song).await;
                            }
                        }
                        KeyCode::Enter => {
                            if app.focus_area == FocusArea::SearchAlbums {
                                let selected = app
                                    .search_albums_liststate
                                    .selected()
                                    .map(|i| app.search_albums[i].clone());
                                if let Some(album) = selected {
                                    fetch_and_play_list(app, client, player, album).await;
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
                                            app.queue = related_songs;
                                            app.queue_liststate.select(Some(0));
                                            app.focus_area = FocusArea::Queue;
                                            app.playing_playlist_id = None;
                                            app.playing_song = None;
                                            if let Err(e) =
                                                player.load_playlist(&app.queue, 0).await
                                            {
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
}

fn handle_lists_event(key_event: KeyEvent, app: &mut App) {
    if matches!(app.popup_state, PopupState::CreatePlaylist { .. }) {
        return;
    }
    let (state, len) = if matches!(app.popup_state, PopupState::SaveSong { .. }) {
        (&mut app.cus_playlists_liststate, app.cus_playlists.len())
    } else {
        match app.focus_area {
            FocusArea::Albums => (&mut app.albums_liststate, app.albums.len()),
            FocusArea::Playlists => (&mut app.playlists_liststate, app.playlists.len()),
            FocusArea::Queue => (&mut app.queue_liststate, app.queue.len()),
            FocusArea::SearchAlbums => (&mut app.search_albums_liststate, app.search_albums.len()),
            FocusArea::SearchSongs => (&mut app.search_songs_liststate, app.search_songs.len()),
            FocusArea::Songs => (&mut app.songs_liststate, app.songs.len()),
        }
    };
    match key_event.code {
        KeyCode::Down | KeyCode::Char('j') => App::next(state, len),
        KeyCode::Up | KeyCode::Char('k') => App::previous(state, len),
        _ => {}
    }
}
async fn handle_queue_event(key_event: KeyEvent, app: &mut App, player: &mut Player) {
    match key_event.code {
        KeyCode::Char('d') => {
            if let Some(i) = app.queue_liststate.selected() {
                if player.play_mode == PlayMode::DefaultMode {
                    if let Err(e) = player.remove_from_queue(i).await {
                        log_to_file(&e);
                    } else {
                        app.queue.remove(i);
                        app.notify(
                            data::NotifyType::Success,
                            String::from("Removed song from Queue"),
                        );
                    }
                } else {
                    let video_id = &app.queue[i].video_id;
                    if let Some(idx_mpv) = app.get_mpv_idx(video_id) {
                        if let Err(e) = player.remove_from_queue(idx_mpv).await {
                            log_to_file(&e);
                        } else {
                            app.queue.remove(i);
                            app.notify(
                                data::NotifyType::Success,
                                String::from("Removed song from Queue"),
                            );
                        }
                    }
                }
                if app.queue.is_empty() {
                    player.status = PlayerStatus::Idle;
                    app.playing_song = None;
                }
            }
        }
        KeyCode::Enter => {
            if let Some(i) = app.queue_liststate.selected() {
                if player.play_mode == PlayMode::DefaultMode {
                    if let Err(e) = player.play_at_idx(&i).await {
                        log_to_file(&e);
                    }
                } else {
                    let target_id = &app.queue[i].video_id;
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
            if app.queue.is_empty() && app.search_songs.is_empty() {
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
            if !app.queue.is_empty()
                && let Err(e) = player.next().await
            {
                log_to_file(&e);
            }
        }
        KeyCode::Char('b') => {
            if !app.queue.is_empty()
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
async fn handle_popup_event(
    key_event: KeyEvent,
    app: &mut App,
    client: Arc<YClient>,
    player: &mut Player,
) {
    match &mut app.popup_state {
        PopupState::SaveSong { selected_save_song } => match key_event.code {
            KeyCode::Esc => {
                app.popup_state = PopupState::None;
            }
            KeyCode::Enter => {
                let song = selected_save_song;
                if let Some(i) = app.cus_playlists_liststate.selected() {
                    if let Some(idx) = app.cus_playlists.get(i) {
                        if let Some(playlist) = app.playlists.get(*idx) {
                            let playlist_id = &playlist.playlist_id;
                            if playlist_id == "LM" {
                                if song.is_liked {
                                    app.notify(
                                        data::NotifyType::Error,
                                        String::from("Song has already been liked"),
                                    );
                                } else {
                                    match client.like_or_unlike_song(&song.video_id, true).await {
                                        Ok(_) => {
                                            if let Some(pos) = app
                                                .search_songs
                                                .iter()
                                                .position(|s| s.video_id == song.video_id)
                                            {
                                                app.search_songs[pos].is_liked = true;
                                            }
                                            if let Some(playing_playlist) = &app.playing_playlist_id
                                            {
                                                if playing_playlist == playlist_id {
                                                    if let Err(e) =
                                                        player.append_to_queue(&song.video_id).await
                                                    {
                                                        log_to_file(&e);
                                                    } else {
                                                        app.queue.push(song.clone());
                                                    }
                                                }
                                            }
                                            app.notify(
                                                data::NotifyType::Success,
                                                String::from("Liked song"),
                                            );
                                        }
                                        Err(e) => {
                                            log_to_file(&e);
                                        }
                                    };
                                }
                            } else {
                                match client.save_to_playlist(&song.video_id, playlist_id).await {
                                    Ok(_) => {
                                        if let Some(playing_playlist) = &app.playing_playlist_id {
                                            if playing_playlist == playlist_id {
                                                if let Err(e) =
                                                    player.append_to_queue(&song.video_id).await
                                                {
                                                    log_to_file(&e);
                                                } else {
                                                    app.queue.push(song.clone());
                                                }
                                            }
                                        }
                                        app.notify(
                                            data::NotifyType::Success,
                                            "Saved song to playlist".to_string(),
                                        );
                                    }
                                    Err(YError::AlreadyInPlaylist) => {
                                        app.notify(
                                            data::NotifyType::Error,
                                            String::from("Song has already been saved in playlist"),
                                        );
                                    }
                                    Err(e) => log_to_file(&e),
                                };
                            }
                        };
                    }
                }
            }
            _ => {}
        },
        PopupState::CreatePlaylist {
            title,
            description,
            privacy,
            focused_field,
        } => match key_event.code {
            KeyCode::Esc => {
                app.popup_state = PopupState::None;
            }
            KeyCode::Enter => {
                if title.is_empty() {
                    app.notify(
                        data::NotifyType::Error,
                        String::from("Title must not be empty"),
                    );
                } else {
                    let result = client.create_playlist(title, description, *privacy).await;
                    match result {
                        Ok(playlist) => {
                            app.playlists.push(playlist);
                            app.cus_playlists.push(app.playlists.len() - 1);
                            app.popup_state = PopupState::None;
                            app.notify(data::NotifyType::Success, "Created playlist".to_string());
                        }
                        Err(e) => {
                            log_to_file(&e);
                            app.notify(
                                data::NotifyType::Error,
                                "Failed to create playlist".to_string(),
                            );
                        }
                    }
                }
            }
            KeyCode::Tab => {
                *focused_field = match focused_field {
                    CreatePlaylistFocus::Title => CreatePlaylistFocus::Description,
                    CreatePlaylistFocus::Description => CreatePlaylistFocus::Privacy,
                    CreatePlaylistFocus::Privacy => CreatePlaylistFocus::Title,
                };
            }
            KeyCode::Left | KeyCode::Char('h')
                if *focused_field == CreatePlaylistFocus::Privacy =>
            {
                *privacy = match *privacy {
                    PlayListPrivacy::Public => PlayListPrivacy::Private,
                    PlayListPrivacy::Unlisted => PlayListPrivacy::Public,
                    PlayListPrivacy::Private => PlayListPrivacy::Unlisted,
                };
            }
            KeyCode::Right | KeyCode::Char('l')
                if *focused_field == CreatePlaylistFocus::Privacy =>
            {
                *privacy = match *privacy {
                    PlayListPrivacy::Public => PlayListPrivacy::Unlisted,
                    PlayListPrivacy::Unlisted => PlayListPrivacy::Private,
                    PlayListPrivacy::Private => PlayListPrivacy::Public,
                };
            }
            KeyCode::Char(c) if *focused_field != CreatePlaylistFocus::Privacy => {
                if *focused_field == CreatePlaylistFocus::Title {
                    title.push(c);
                } else {
                    description.push(c);
                }
            }
            KeyCode::Backspace if *focused_field != CreatePlaylistFocus::Privacy => {
                if *focused_field == CreatePlaylistFocus::Title {
                    title.pop();
                } else {
                    description.pop();
                }
            }
            _ => {}
        },
        PopupState::None => {}
    }
}
async fn append_song_to_queue(app: &mut App, player: &mut Player, song: Song) {
    if let Err(e) = player.append_to_queue(&song.video_id).await {
        log_to_file(&e);
    } else {
        let new_song = song.clone();
        app.queue.push(new_song);
        app.notify(
            data::NotifyType::Success,
            String::from("Added song to Queue"),
        );
    }
}
async fn fetch_and_play_list(
    app: &mut App,
    client: Arc<YClient>,
    player: &mut Player,
    list: PlayList,
) {
    let browse_id = &list.browse_id;
    let playlist_id = &list.playlist_id;
    if let Ok(songs) = client.get_songs(browse_id).await {
        if !songs.is_empty() {
            app.queue = songs;
            app.queue_liststate.select(Some(0));
            app.focus_area = FocusArea::Queue;
            app.playing_playlist_id = Some(playlist_id.clone());
            app.playing_song = None;
            if let Err(e) = player.load_playlist(&app.queue, 0).await {
                log_to_file(&e);
            }
        }
    } else {
        log_to_file("Fetching songs Error");
    }
}
