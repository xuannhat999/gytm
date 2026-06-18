use crate::{
    app::{App, PopupState},
    helper::{self, get_url_from_vid_id},
};
use api::YClient;
use crossterm::event::{KeyCode, KeyEvent};
use data::{
    AppPage, CreatePlaylistFocus, FocusArea, MpvCommand, MpvEvent, PlayListPrivacy, PlayMode,
    PlayerStatus::{self},
    Song,
};
use error::{YError, YResult, log_to_file};
use player::Player;
use state::AppState;

pub fn handle_mpv_event(app: &mut App, state: &mut AppState, event: MpvEvent) {
    match event {
        MpvEvent::ListChange(list) => {
            let ids = helper::list_vid_id_from_list_url(list);
            app.mpv_list = ids;
        }
        MpvEvent::StartPlaying(url) => {
            let video_id = helper::get_vid_id_from_url(&url);
            for song in &app.queue {
                if song.video_id == video_id {
                    app.playing_song = Some(song.clone());
                    app.time_pos = Some(0.0);
                }
            }
            app.status = PlayerStatus::Playing;
        }
        MpvEvent::VolumeChange(vol) => {
            app.volume = vol;
            state.player_state.volume = vol;
            if let Err(e) = state.save() {
                log_to_file(&e);
            }
        }
        MpvEvent::TimePos(pos) => {
            app.time_pos = Some(pos);
        }
        MpvEvent::PauseChange(is_pause) => {
            if app.playing_song.is_some() {
                if is_pause {
                    app.status = PlayerStatus::Paused
                } else {
                    app.status = PlayerStatus::Playing
                }
            }
        }
    }
}
pub async fn handle_key_events(
    key_event: KeyEvent,
    app: &mut App,
    client: &YClient,
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
        if !app.is_insert {
            match key_event.code {
                KeyCode::Char('q') => {
                    if app.queue.is_empty() {
                        App::shutdown(player).await;
                    } else {
                        let _ = app.save_queue_file();
                    }
                    app.is_exit = true;
                }
                KeyCode::Char('Q') => {
                    App::shutdown(player).await;
                    app.is_exit = true;
                }
                KeyCode::Char('3') => {
                    app.focus_area = FocusArea::Queue;
                    if app.queue_liststate.selected().is_none() && !app.queue.is_empty() {
                        app.queue_liststate.select(Some(0));
                    }
                }
                KeyCode::Char('4') => {
                    app.focus_area = FocusArea::Songs;
                }
                KeyCode::Char('c') => {
                    if let Err(e) = clear_queue(app, player) {
                        log_to_file(&e);
                    }
                }
                _ => {}
            }
            match app.focus_area {
                FocusArea::Queue => {
                    handle_queue_event(key_event, app, player).await;
                }
                FocusArea::Songs => {
                    handle_songs_event(key_event, app, client, player).await;
                }
                _ => {}
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
                            let browse_id = &list.browse_id;
                            let playlist_id = list.playlist_id;
                            if let Ok(songs) = client.get_songs(browse_id).await {
                                if let Err(e) =
                                    load_list(app, player, songs, 0, Some(playlist_id)).await
                                {
                                    log_to_file(&e);
                                }
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
                                            app.refresh_cus_playlist();
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
                    _ => {}
                },
                KeyCode::Char('a') => {
                    if app.focus_area == FocusArea::Playlists {
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
                            if app.focus_area == FocusArea::SearchSongs {
                                if let Some(song) = app
                                    .search_songs_liststate
                                    .selected()
                                    .map(|i| app.search_songs[i].clone())
                                {
                                    if let Err(e) = append_song_to_queue(app, player, &song).await {
                                        log_to_file(&e);
                                    }
                                }
                            }
                        }
                        KeyCode::Enter => {
                            if app.focus_area == FocusArea::SearchAlbums {
                                let selected = app
                                    .search_albums_liststate
                                    .selected()
                                    .map(|i| app.search_albums[i].clone());
                                if let Some(album) = selected {
                                    let browse_id = &album.browse_id;
                                    let playlist_id = album.playlist_id;
                                    if let Ok(songs) = client.get_songs(browse_id).await {
                                        if let Err(e) =
                                            load_list(app, player, songs, 0, Some(playlist_id))
                                                .await
                                        {
                                            log_to_file(&e);
                                        };
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
                                            if let Err(e) =
                                                load_list(app, player, related_songs, 0, None).await
                                            {
                                                log_to_file(&e);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        KeyCode::Char('l') => {
                            if app.focus_area == FocusArea::SearchAlbums {
                                let list = app
                                    .search_albums_liststate
                                    .selected()
                                    .map(|i| &app.search_albums[i]);
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
        KeyCode::Down | KeyCode::Char('j') => App::next_item(state, len),
        KeyCode::Up | KeyCode::Char('k') => App::previous_item(state, len),
        _ => {}
    }
}
async fn handle_queue_event(key_event: KeyEvent, app: &mut App, player: &mut Player) {
    match key_event.code {
        KeyCode::Char('d') => {
            if let Some(i) = app.queue_liststate.selected() {
                if app.play_mode == PlayMode::DefaultMode {
                    remove_song_from_queue(app, player, i, i).await;
                } else {
                    let video_id = &app.queue[i].video_id;
                    if let Some(idx_mpv) = app.get_mpv_idx(video_id) {
                        remove_song_from_queue(app, player, i, idx_mpv).await;
                    }
                }
                if app.queue.is_empty() {
                    app.status = PlayerStatus::Idle;
                    app.playing_song = None;
                    app.playing_playlist_id = None;
                    app.time_pos = None;
                }
            }
        }
        KeyCode::Enter => {
            if let Some(i) = app.queue_liststate.selected() {
                if app.play_mode == PlayMode::DefaultMode {
                    if let Err(e) = player.send_mpv_command(MpvCommand::PlayPos(i)) {
                        log_to_file(&e);
                    }
                } else {
                    let target_id = &app.queue[i].video_id;
                    if let Some(pos) = app.get_mpv_idx(target_id) {
                        if let Err(e) = player.send_mpv_command(MpvCommand::PlayPos(pos)) {
                            log_to_file(&e);
                        } else {
                            if let Err(e) = player.send_mpv_command(MpvCommand::Shuffle) {
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
            app.is_insert = false;
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
            if let Err(e) = player.send_mpv_command(MpvCommand::TogglePause) {
                log_to_file(&e);
            }
        }
        KeyCode::Char('m') => {
            let res = match app.play_mode {
                PlayMode::DefaultMode => {
                    app.play_mode = PlayMode::ShuffleMode;
                    player.send_mpv_command(MpvCommand::Shuffle)
                }
                PlayMode::ShuffleMode => {
                    app.play_mode = PlayMode::DefaultMode;
                    player.send_mpv_command(MpvCommand::Unshuffle)
                }
            };
            if let Err(e) = res {
                log_to_file(&e);
            } else {
                state.player_state.play_mode = app.play_mode.clone();
                if let Err(e) = state.save() {
                    log_to_file(&e);
                }
            }
        }
        KeyCode::Char('n') => {
            if !app.queue.is_empty()
                && let Err(e) = player.send_mpv_command(MpvCommand::PlayNext)
            {
                log_to_file(&e);
            }
        }
        KeyCode::Char('b') => {
            if !app.queue.is_empty()
                && let Err(e) = player.send_mpv_command(MpvCommand::PlayPrev)
            {
                log_to_file(&e);
            }
        }
        KeyCode::Char('-') => {
            if let Err(e) = player.send_mpv_command(MpvCommand::DecreaseVol) {
                log_to_file(&e);
            }
        }
        KeyCode::Char('+') => {
            if let Err(e) = player.send_mpv_command(MpvCommand::IncreaseVol) {
                log_to_file(&e);
            }
        }
        KeyCode::Left => {
            if let Err(e) = player.send_mpv_command(MpvCommand::SeekBackward) {
                log_to_file(&e);
            }
        }
        KeyCode::Right => {
            if let Err(e) = player.send_mpv_command(MpvCommand::SeekForward) {
                log_to_file(&e);
            }
        }
        _ => {}
    }
}
async fn handle_songs_event(
    key_event: KeyEvent,
    app: &mut App,
    client: &YClient,
    player: &mut Player,
) {
    match key_event.code {
        KeyCode::Enter => {
            if let Some(list) = &app.viewing_list {
                let is_dup = app.playing_playlist_id.as_ref() == Some(&list.playlist_id);
                if !is_dup {
                    if let Some(i) = app.songs_liststate.selected() {
                        if let Err(e) = load_list(
                            app,
                            player,
                            app.songs.clone(),
                            i,
                            Some(list.playlist_id.clone()),
                        )
                        .await
                        {
                            log_to_file(&e);
                        }
                    }
                } else {
                    app.notify(
                        data::NotifyType::Error,
                        String::from("Playlist/Album's already been playing, change song in Queue"),
                    );
                }
            }
        }
        KeyCode::Char('a') => {
            if let Some(song) = app.songs_liststate.selected().map(|i| app.songs[i].clone()) {
                if let Err(e) = append_song_to_queue(app, player, &song).await {
                    log_to_file(&e);
                }
            }
        }
        KeyCode::Char('X') => {
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
        KeyCode::Char('x') => {
            if let Some(i) = app.songs_liststate.selected() {
                if let Some(song) = app.songs.get(i) {
                    app.popup_state = PopupState::SaveSong {
                        selected_save_song: song.clone(),
                    };
                    app.cus_playlists_liststate.select(Some(0));
                }
            }
        }

        _ => {}
    }
}
async fn handle_popup_event(
    key_event: KeyEvent,
    app: &mut App,
    client: &YClient,
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
                                match client.like_or_unlike_song(&song.video_id, true).await {
                                    Ok(_) => {
                                        if let Some(playing_playlist) = &app.playing_playlist_id {
                                            if playing_playlist == playlist_id {
                                                let url =
                                                    helper::get_url_from_vid_id(&song.video_id);
                                                if let Err(e) = player
                                                    .send_mpv_command(MpvCommand::AppendSong(url))
                                                {
                                                    log_to_file(&e);
                                                } else {
                                                    app.queue.push(song.clone());
                                                }
                                            }
                                        }
                                        if let Some(viewing_list) = &app.viewing_list
                                            && viewing_list.playlist_id.eq(playlist_id)
                                        {
                                            app.songs.push(song.clone());
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
                            } else {
                                match client.save_to_playlist(&song.video_id, playlist_id).await {
                                    Ok(_) => {
                                        if let Some(playing_playlist) = &app.playing_playlist_id {
                                            if playing_playlist == playlist_id {
                                                let url = get_url_from_vid_id(&song.video_id);
                                                if let Err(e) = player
                                                    .send_mpv_command(MpvCommand::AppendSong(url))
                                                {
                                                    log_to_file(&e);
                                                } else {
                                                    app.queue.push(song.clone());
                                                }
                                            }
                                        }
                                        if let Some(viewing_list) = &app.viewing_list
                                            && viewing_list.playlist_id.eq(playlist_id)
                                        {
                                            app.songs.push(song.clone());
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

async fn append_song_to_queue(app: &mut App, player: &mut Player, song: &Song) -> YResult<()> {
    let url = get_url_from_vid_id(&song.video_id);
    player.send_mpv_command(MpvCommand::AppendSong(url))?;
    let new_song = song.clone();
    app.queue.push(new_song);
    app.notify(
        data::NotifyType::Success,
        String::from("Added song to Queue"),
    );
    if app.play_mode == PlayMode::ShuffleMode && app.queue.len() == 3 {
        player.send_mpv_command(MpvCommand::Shuffle)?;
    }
    Ok(())
}

async fn remove_song_from_queue(app: &mut App, player: &mut Player, idx: usize, mpv_idx: usize) {
    if let Err(e) = player.send_mpv_command(MpvCommand::RemovePos(mpv_idx)) {
        log_to_file(&e);
    } else {
        app.queue.remove(idx);
        app.notify(
            data::NotifyType::Success,
            String::from("Removed song from Queue"),
        );
    }
}

fn clear_queue(app: &mut App, player: &mut Player) -> YResult<()> {
    player.send_mpv_command(MpvCommand::Stop)?;
    player.send_mpv_command(MpvCommand::Clear)?;
    app.status = PlayerStatus::Idle;
    app.playing_song = None;
    app.time_pos = None;
    app.queue = Vec::new();
    app.playing_playlist_id = None;
    app.notify(data::NotifyType::Success, String::from("Cleared Queue"));
    Ok(())
}

async fn load_list(
    app: &mut App,
    player: &mut Player,
    songs: Vec<Song>,
    start_index: usize,
    playlist_id: Option<String>,
) -> YResult<()> {
    player.write_playlist(&songs)?;
    player.send_mpv_command(MpvCommand::LoadList)?;
    if start_index > 0 {
        player.send_mpv_command(MpvCommand::PlayPos(start_index))?;
    }
    if app.play_mode == PlayMode::ShuffleMode {
        player.send_mpv_command(MpvCommand::Shuffle)?;
    }

    app.queue = songs;
    app.queue_liststate.select(Some(start_index));
    app.focus_area = FocusArea::Queue;
    app.playing_playlist_id = playlist_id;
    app.playing_song = None;
    Ok(())
}
