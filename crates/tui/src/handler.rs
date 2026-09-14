use crate::{
    app::App,
    helper::{self, get_url_from_vid_id},
    notification::NotifyType,
};
use api::{
    client::{self, gecko::get_geckgo_containers_from_profile, get_profiles_from_browser},
    protocol::{ApiCmd, ApiLoadingKind, ApiResponse},
};
use config::Config;
use crossterm::event::{KeyCode, KeyEvent};
use data::{
    api_client::{ALL_BROWSERS, BrowserEngine},
    app::{
        AppPage, CreatePlaylistFocus,
        FocusArea::{self},
        PlayListPrivacy, PlayMode,
        PlayerStatus::{self},
        PopupState, Song,
    },
    mpv::{MpvCommand, MpvEvent},
};
use error::{YError, YResult, log_to_file};
use player::Player;
use ratatui::widgets::ListState;
use state::{Persist, client_state::ClientState};
use std::fs;

pub fn handle_mpv_event(app: &mut App, event: MpvEvent) {
    match event {
        MpvEvent::ListChange(list) => {
            let ids = helper::list_vid_id_from_list_url(list);
            app.mpv_list = ids;
            app.save_queue_file().ok();
            fs::remove_file(data::file_path::MPV_PLAYLIST).ok();
        }
        MpvEvent::StartPlaying(url) => {
            let video_id = helper::get_vid_id_from_url(&url);
            let idx = app.queue.iter().position(|song| song.video_id == video_id);
            if idx != app.playing_song {
                app.player_status = PlayerStatus::Playing;
                app.time_pos = Some(0.0);
            }
            app.playing_song = idx;
        }
        MpvEvent::VolumeChange(vol) => {
            app.player_state.volume = vol;
            if let Err(e) = app.player_state.save() {
                log_to_file(&e);
            }
        }
        MpvEvent::TimePos(pos) => {
            app.time_pos = Some(pos);
        }
        MpvEvent::PauseChange(is_pause) => {
            if app.playing_song.is_some() {
                if is_pause {
                    app.player_status = PlayerStatus::Paused
                } else {
                    app.player_status = PlayerStatus::Playing
                }
            }
        }
    }
}
pub fn handle_key_events(key_event: KeyEvent, app: &mut App, player: &mut Player, config: &Config) {
    if (!app.is_popup_active() && !app.is_insert)
        || matches!(app.popup_state, PopupState::SaveSong { .. })
        || matches!(app.popup_state, PopupState::SelectBrowser)
        || matches!(app.popup_state, PopupState::SelectBrowserProfile { .. })
        || matches!(app.popup_state, PopupState::SelectGeckoContainer { .. })
        || matches!(app.popup_state, PopupState::SelectAccount { .. })
    {
        handle_lists_event(key_event, app);
    }

    if app.is_popup_active() {
        handle_popup_event(key_event, app);
    } else {
        if key_event.code == KeyCode::Tab {
            handle_page_event(app);
        }
        if !app.is_insert {
            match key_event.code {
                KeyCode::Char('q') => {
                    if app.queue.is_empty() {
                        App::shutdown(player);
                    }
                    app.is_exit = true;
                }
                KeyCode::Char('Q') => {
                    App::shutdown(player);
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
                    } else {
                        app.noti
                            .notify(NotifyType::Success, String::from("Cleared Queue"));
                    }
                }
                KeyCode::Char('B') => {
                    app.popup_state = PopupState::SelectBrowser;
                    app.browser_liststate.select(Some(0));
                }
                KeyCode::Char('P') => match app.client_state.get_validated_fields() {
                    Ok((browser, _, _, _)) => match get_profiles_from_browser(&browser) {
                        Ok(profiles) => {
                            let mut profiles_liststate = ListState::default();
                            if !profiles.is_empty() {
                                profiles_liststate.select(Some(0));
                            }
                            app.popup_state = PopupState::SelectBrowserProfile {
                                browser,
                                profiles,
                                profiles_liststate,
                            };
                        }
                        Err(e) => log_to_file(e),
                    },
                    Err(_) => {
                        app.popup_state = PopupState::SelectBrowser;
                        app.browser_liststate.select(Some(0));
                    }
                },
                KeyCode::Char('A') => match app.client_state.get_validated_fields() {
                    Ok((browser, profile, container, _)) => {
                        let client = ClientState {
                            browser: Some(browser),
                            profile: Some(profile),
                            gecko_container: container,
                            account: None,
                        };
                        app.api_loading_kind = Some(ApiLoadingKind::FetchAccountsList);
                        app.api_cmd_tx.send(ApiCmd::FetchAccountsList(client)).ok();
                    }
                    Err(_) => {
                        app.popup_state = PopupState::SelectBrowser;
                        app.browser_liststate.select(Some(0));
                    }
                },
                KeyCode::Char('C') => {
                    if let Ok((browser, profile, _, _)) = app.client_state.get_validated_fields()
                        && browser.engine() == BrowserEngine::Gecko
                        && let Ok(containers) = get_geckgo_containers_from_profile(&profile.path)
                    {
                        let mut containers_liststate = ListState::default();
                        containers_liststate.select(Some(0));
                        app.popup_state = PopupState::SelectGeckoContainer {
                            browser,
                            profile,
                            containers,
                            containers_liststate,
                        };
                    }
                }
                _ => {}
            }
            match app.focus_area {
                FocusArea::Queue => {
                    handle_queue_event(key_event, app, player);
                }
                FocusArea::Songs => {
                    handle_songs_event(key_event, app, player);
                }
                _ => {}
            }
            handle_player_event(key_event, app, player, config);
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
                        app.api_cmd_tx
                            .send(ApiCmd::GetSongsToView(list.clone()))
                            .ok();
                        app.focus_area = FocusArea::Songs;
                        app.api_loading_kind = Some(ApiLoadingKind::GetSongsToView);
                    }
                }
                KeyCode::Enter => match app.focus_area {
                    FocusArea::Albums | FocusArea::Playlists => {
                        let is_album = app.focus_area == FocusArea::Albums;
                        let selection = if is_album {
                            app.albums_liststate.selected().map(|i| &app.albums[i])
                        } else {
                            app.playlists_liststate
                                .selected()
                                .map(|i| &app.playlists[i])
                        };
                        if let Some(list) = selection {
                            app.api_cmd_tx
                                .send(ApiCmd::GetSongsToPlay(list.clone()))
                                .ok();
                            app.api_loading_kind = Some(ApiLoadingKind::GetSongsToPlay);
                            app.focus_area = FocusArea::Queue;
                        }
                    }
                    _ => {}
                },
                KeyCode::Char('x') => match app.focus_area {
                    FocusArea::Albums => {
                        if let Some(i) = app.albums_liststate.selected() {
                            if let Some(album) = app.albums.get(i) {
                                app.api_cmd_tx.send(ApiCmd::UnsaveAlbum(album.clone())).ok();
                                if let Some(pos) = app
                                    .search_albums
                                    .iter()
                                    .position(|a| album.playlist_id == a.playlist_id)
                                {
                                    app.search_albums[pos].is_saved = false;
                                }
                                app.albums.remove(i);
                            }
                        }
                    }
                    FocusArea::Playlists => {
                        if let Some(i) = app.playlists_liststate.selected() {
                            if let Some(playlist) = app.playlists.get(i) {
                                if playlist.playlist_id == "LM" || playlist.playlist_id == "SE" {
                                    app.noti.notify(
                                        NotifyType::Error,
                                        String::from("Can not remove this playlist"),
                                    );
                                } else {
                                    if playlist.is_custom {
                                        app.api_cmd_tx
                                            .send(ApiCmd::UnsaveCusPlaylist(playlist.clone()))
                                            .ok();
                                    } else {
                                        app.api_cmd_tx
                                            .send(ApiCmd::UnsaveAlbum(playlist.clone()))
                                            .ok();
                                    };
                                    app.playlists.remove(i);
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
                            app.api_cmd_tx
                                .send(ApiCmd::Search(app.search_query.clone()))
                                .ok();
                            app.api_loading_kind = Some(ApiLoadingKind::Search);
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
                        KeyCode::Char('x') => {
                            match app.focus_area {
                                FocusArea::SearchAlbums => {
                                    if let Some(i) = app.search_albums_liststate.selected() {
                                        if let Some(selected) = app.search_albums.get_mut(i) {
                                            if !selected.is_saved {
                                                selected.is_saved = true;
                                                app.api_cmd_tx
                                                    .send(ApiCmd::SaveAlbum(selected.clone()))
                                                    .ok();
                                            } else {
                                                app.api_cmd_tx
                                                    .send(ApiCmd::UnsaveAlbum(selected.clone()))
                                                    .ok();
                                                selected.is_saved = false;
                                                if let Some(idx) = app.albums.iter().position(|a| {
                                                    a.playlist_id == selected.playlist_id
                                                }) {
                                                    app.albums.remove(idx);
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
                            }
                        }
                        KeyCode::Char('a') => {
                            if app.focus_area == FocusArea::SearchSongs {
                                if let Some(song) = app
                                    .search_songs_liststate
                                    .selected()
                                    .map(|i| app.search_songs[i].clone())
                                {
                                    if let Err(e) = append_song_to_queue(app, player, song) {
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
                                    .map(|i| &app.search_albums[i]);
                                if let Some(album) = selected {
                                    app.api_cmd_tx
                                        .send(ApiCmd::GetSongsToPlay(album.clone()))
                                        .ok();
                                    app.api_loading_kind = Some(ApiLoadingKind::GetSongsToPlay);
                                }
                            } else if app.focus_area == FocusArea::SearchSongs {
                                let selected = app
                                    .search_songs_liststate
                                    .selected()
                                    .map(|i| &app.search_songs[i]);
                                if let Some(song) = selected {
                                    app.api_cmd_tx
                                        .send(ApiCmd::GetRelatedSongsToPlay(song.clone()))
                                        .ok();
                                    app.api_loading_kind = Some(ApiLoadingKind::GetSongsToPlay);
                                }
                            }
                            app.focus_area = FocusArea::Queue;
                        }
                        KeyCode::Char('l') => {
                            if app.focus_area == FocusArea::SearchAlbums {
                                let list = app
                                    .search_albums_liststate
                                    .selected()
                                    .map(|i| &app.search_albums[i]);
                                if let Some(list) = list {
                                    app.api_cmd_tx
                                        .send(ApiCmd::GetSongsToView(list.clone()))
                                        .ok();

                                    app.api_loading_kind = Some(ApiLoadingKind::GetSongsToView);
                                    app.focus_area = FocusArea::Songs;
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
    let (state, len) = if matches!(app.popup_state, PopupState::SaveSong { .. }) {
        (&mut app.cus_playlists_liststate, app.cus_playlists.len())
    } else if matches!(app.popup_state, PopupState::SelectBrowser) {
        (&mut app.browser_liststate, ALL_BROWSERS.len())
    } else if let PopupState::SelectBrowserProfile {
        profiles,
        profiles_liststate,
        ..
    } = &mut app.popup_state
    {
        (profiles_liststate, profiles.len())
    } else if let PopupState::SelectGeckoContainer {
        containers,
        containers_liststate,
        ..
    } = &mut app.popup_state
    {
        (containers_liststate, containers.len())
    } else if let PopupState::SelectAccount {
        accounts,
        accounts_liststate,
        ..
    } = &mut app.popup_state
    {
        (accounts_liststate, accounts.len())
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

fn handle_queue_event(key_event: KeyEvent, app: &mut App, player: &mut Player) {
    match key_event.code {
        KeyCode::Char('d') => {
            if let Some(i) = app.queue_liststate.selected() {
                if app.player_state.play_mode == PlayMode::DefaultMode {
                    remove_song_from_queue(app, player, i, i);
                } else {
                    let video_id = &app.queue[i].video_id;
                    if let Some(idx_mpv) = app.get_mpv_idx(video_id) {
                        remove_song_from_queue(app, player, i, idx_mpv);
                    }
                }
            }
        }
        KeyCode::Enter => {
            if let Some(i) = app.queue_liststate.selected() {
                if app.player_state.play_mode == PlayMode::DefaultMode {
                    if let Err(e) = player.send_mpv_command(MpvCommand::PlayPos(i)) {
                        log_to_file(&e);
                    }
                } else {
                    let video_id = &app.queue[i].video_id;
                    if let Some(pos) = app.get_mpv_idx(video_id)
                        && let Err(e) = player.send_mpv_command(MpvCommand::PlayPos(pos))
                    {
                        log_to_file(&e);
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
fn handle_player_event(key_event: KeyEvent, app: &mut App, player: &mut Player, config: &Config) {
    match key_event.code {
        KeyCode::Char(' ') if app.playing_song.is_some() => {
            if let Err(e) = player.send_mpv_command(MpvCommand::TogglePause) {
                log_to_file(&e);
            }
        }
        KeyCode::Char('m') => {
            let res = match app.player_state.play_mode {
                PlayMode::DefaultMode => {
                    app.player_state.play_mode = PlayMode::ShuffleMode;
                    player.send_mpv_command(MpvCommand::Shuffle)
                }
                PlayMode::ShuffleMode => {
                    app.player_state.play_mode = PlayMode::DefaultMode;
                    player.send_mpv_command(MpvCommand::Unshuffle)
                }
            };
            if let Err(e) = res {
                log_to_file(&e);
            } else if let Err(e) = app.player_state.save() {
                log_to_file(&e);
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
            if let Err(e) =
                player.send_mpv_command(MpvCommand::SeekBackward(config.seek_seconds as i64))
            {
                log_to_file(&e);
            }
        }
        KeyCode::Right => {
            if let Err(e) =
                player.send_mpv_command(MpvCommand::SeekForward(config.seek_seconds as i64))
            {
                log_to_file(&e);
            }
        }
        _ => {}
    }
}
fn handle_songs_event(key_event: KeyEvent, app: &mut App, player: &mut Player) {
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
                        ) {
                            log_to_file(&e);
                        } else {
                            app.focus_area = FocusArea::Queue;
                        }
                    }
                } else {
                    app.noti.notify(
                        NotifyType::Error,
                        String::from("Playlist/Album's already been playing, change song in Queue"),
                    );
                }
            }
        }
        KeyCode::Char('a') => {
            if let Some(song) = app.songs_liststate.selected().map(|i| app.songs[i].clone()) {
                if let Err(e) = append_song_to_queue(app, player, song) {
                    log_to_file(&e);
                }
            }
        }
        KeyCode::Char('X') => {
            if let Some(list) = &app.viewing_list {
                if list.is_custom {
                    if let Some(i) = app.songs_liststate.selected() {
                        let song = &app.songs[i];
                        if list.playlist_id == "LM" {
                            app.api_cmd_tx.send(ApiCmd::UnlikeSong(song.clone())).ok()
                        } else {
                            app.api_cmd_tx
                                .send(ApiCmd::UnsaveSong {
                                    song: song.clone(),
                                    playlist_id: list.playlist_id.clone(),
                                })
                                .ok()
                        };

                        app.songs.remove(i);
                    }
                } else {
                    app.noti.notify(
                        NotifyType::Error,
                        String::from("Unable to edit this Album/Playlist"),
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
fn handle_popup_event(key_event: KeyEvent, app: &mut App) {
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
                                app.api_cmd_tx.send(ApiCmd::LikeSong(song.clone())).ok();
                            } else {
                                app.api_cmd_tx
                                    .send(ApiCmd::SaveSong {
                                        song: song.clone(),
                                        playlist_id: playlist_id.clone(),
                                    })
                                    .ok();
                            }
                            app.api_loading_kind = Some(ApiLoadingKind::SaveToPlaylist)
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
                    app.noti
                        .notify(NotifyType::Error, String::from("Title must not be empty"));
                } else {
                    app.api_cmd_tx
                        .send(ApiCmd::CreatePlaylist {
                            title: title.clone(),
                            description: description.clone(),
                            privacy: *privacy,
                        })
                        .ok();
                    app.api_loading_kind = Some(ApiLoadingKind::CreatePlaylist);
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
        PopupState::SelectBrowser => match key_event.code {
            KeyCode::Esc => app.popup_state = PopupState::None,
            KeyCode::Char('h') | KeyCode::Left => app.popup_state = PopupState::None,
            KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
                if let Some(index) = app.browser_liststate.selected() {
                    let browser = &ALL_BROWSERS[index];
                    match client::get_profiles_from_browser(browser) {
                        Ok(profiles) => {
                            let mut profiles_liststate = ListState::default();
                            if !profiles.is_empty() {
                                profiles_liststate.select(Some(0));
                            }
                            app.popup_state = PopupState::SelectBrowserProfile {
                                profiles,
                                profiles_liststate,
                                browser: *browser,
                            }
                        }
                        Err(e) => {
                            log_to_file(e);
                        }
                    }
                }
            }
            _ => {}
        },
        PopupState::SelectBrowserProfile {
            profiles,
            profiles_liststate,
            browser,
        } => match key_event.code {
            KeyCode::Esc => app.popup_state = PopupState::None,
            KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
                if let Some(index) = profiles_liststate.selected() {
                    let profile = &profiles[index];
                    match browser.engine() {
                        BrowserEngine::Gecko => {
                            if let Ok(containers) =
                                get_geckgo_containers_from_profile(&profile.path)
                            {
                                let mut containers_liststate = ListState::default();
                                containers_liststate.select(Some(0));
                                app.popup_state = PopupState::SelectGeckoContainer {
                                    browser: *browser,
                                    profile: profile.clone(),
                                    containers,
                                    containers_liststate,
                                };
                            }
                        }
                        BrowserEngine::Chromium => {
                            // app.client_state.gecko_container = None;
                            // if let Err(e) = app.client_state.save() {
                            //     log_to_file(e);
                            // }
                            // reaload_api_client(app);
                            // app.popup_state = PopupState::None;
                            let client = ClientState {
                                browser: Some(*browser),
                                profile: Some(profile.clone()),
                                gecko_container: None,
                                account: None,
                            };
                            app.api_loading_kind = Some(ApiLoadingKind::FetchAccountsList);
                            app.api_cmd_tx.send(ApiCmd::FetchAccountsList(client)).ok();
                        }
                    }
                }
            }
            KeyCode::Char('h') | KeyCode::Left => app.popup_state = PopupState::SelectBrowser,
            _ => {}
        },
        PopupState::SelectGeckoContainer {
            containers,
            containers_liststate,
            browser,
            profile,
        } => match key_event.code {
            KeyCode::Esc => app.popup_state = PopupState::None,
            KeyCode::Char('h') | KeyCode::Left => {
                match client::get_profiles_from_browser(browser) {
                    Ok(profiles) => {
                        let mut profiles_liststate = ListState::default();
                        if !profiles.is_empty() {
                            profiles_liststate.select(Some(0));
                        }
                        app.popup_state = PopupState::SelectBrowserProfile {
                            browser: *browser,
                            profiles,
                            profiles_liststate,
                        }
                    }
                    Err(e) => {
                        log_to_file(e);
                    }
                }
            }
            KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
                if let Some(index) = containers_liststate.selected() {
                    let container = &containers[index];
                    let client = ClientState {
                        browser: Some(*browser),
                        profile: Some(profile.clone()),
                        gecko_container: Some(container.clone()),
                        account: None,
                    };
                    // if let Err(e) = app.client_state.save() {
                    //     log_to_file(e);
                    // }
                    // reaload_api_client(app);
                    // app.popup_state = PopupState::None;
                    app.api_loading_kind = Some(ApiLoadingKind::FetchAccountsList);
                    app.api_cmd_tx.send(ApiCmd::FetchAccountsList(client)).ok();
                }
            }
            _ => {}
        },
        PopupState::SelectAccount {
            browser,
            profile,
            container,
            accounts,
            accounts_liststate,
        } => match key_event.code {
            KeyCode::Esc => app.popup_state = PopupState::None,
            KeyCode::Char('h') | KeyCode::Left => {
                if container.is_some() {
                    match get_geckgo_containers_from_profile(&profile.path) {
                        Ok(containers) => {
                            let mut liststate = ListState::default();
                            if !containers.is_empty() {
                                liststate.select(Some(0));
                            }
                            app.popup_state = PopupState::SelectGeckoContainer {
                                browser: *browser,
                                profile: profile.clone(),
                                containers,
                                containers_liststate: liststate,
                            };
                        }
                        Err(e) => {
                            log_to_file(e);
                        }
                    }
                } else {
                    match get_profiles_from_browser(browser) {
                        Ok(profiles) => {
                            let mut liststate = ListState::default();
                            if !profiles.is_empty() {
                                liststate.select(Some(0));
                            }
                            app.popup_state = PopupState::SelectBrowserProfile {
                                browser: *browser,
                                profiles,
                                profiles_liststate: liststate,
                            };
                        }
                        Err(e) => {
                            log_to_file(&e);
                        }
                    }
                }
            }
            KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
                if let Some(index) = accounts_liststate.selected() {
                    let selected_acc = &accounts[index];
                    let client_state = ClientState {
                        browser: Some(*browser),
                        profile: Some(profile.clone()),
                        gecko_container: container.clone(),
                        account: Some(selected_acc.clone()),
                    };
                    app.albums = Vec::new();
                    app.playlists = Vec::new();
                    app.api_cmd_tx
                        .send(ApiCmd::ReloadApiClient(client_state))
                        .ok();
                    app.api_loading_kind = Some(ApiLoadingKind::FetchLibraryData);
                    app.popup_state = PopupState::None;
                }
            }
            _ => {}
        },
        _ => {}
    }
}

fn append_song_to_queue(app: &mut App, player: &Player, song: Song) -> YResult<()> {
    if app.queue.iter().any(|s| s.video_id == song.video_id) {
        app.noti.notify(
            NotifyType::Error,
            format!("'{}' already in queue", song.title),
        );
        return Ok(());
    }
    let url = get_url_from_vid_id(&song.video_id);
    player.send_mpv_command(MpvCommand::AppendSong(url))?;
    if app.player_state.play_mode == PlayMode::ShuffleMode && app.queue.len() == 3 {
        player.send_mpv_command(MpvCommand::Shuffle)?;
    }
    app.noti.notify(
        NotifyType::Success,
        format!("Appended '{}' in queue", song.title),
    );
    app.queue.push(song);

    Ok(())
}

fn remove_song_from_queue(app: &mut App, player: &mut Player, idx: usize, mpv_idx: usize) {
    if let Err(e) = player.send_mpv_command(MpvCommand::RemovePos(mpv_idx)) {
        log_to_file(&e);
    } else {
        if let Some(playing_idx) = app.playing_song {
            if playing_idx > idx {
                app.playing_song = Some(playing_idx - 1);
            } else if playing_idx == idx {
                app.playing_song = None;
            }
        }
        app.queue.remove(idx);
        if app.queue.is_empty() {
            app.player_status = PlayerStatus::Idle;
            app.playing_playlist_id = None;
            app.time_pos = None;
        }
        app.noti
            .notify(NotifyType::Success, String::from("Removed song from Queue"));
    }
}

fn clear_queue(app: &mut App, player: &Player) -> YResult<()> {
    player.send_mpv_command(MpvCommand::Clear)?;
    app.player_status = PlayerStatus::Idle;
    app.playing_song = None;
    app.time_pos = None;
    app.queue = Vec::new();
    app.playing_playlist_id = None;

    Ok(())
}

fn load_list(
    app: &mut App,
    player: &Player,
    songs: Vec<Song>,
    start_index: usize,
    playlist_id: Option<String>,
) -> YResult<()> {
    if !songs.is_empty() {
        player.write_playlist(&songs)?;
        player.send_mpv_command(MpvCommand::LoadList)?;
        if start_index > 0 {
            player.send_mpv_command(MpvCommand::PlayPos(start_index))?;
        }
        if app.player_state.play_mode == PlayMode::ShuffleMode {
            player.send_mpv_command(MpvCommand::Shuffle)?;
        }
        app.queue = songs;
        app.queue_liststate.select(Some(start_index));
        app.playing_playlist_id = playlist_id;
        app.playing_song = None;
    } else {
        clear_queue(app, player).ok();
    }
    Ok(())
}

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
        ApiResponse::Search { albums, songs } => {
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
                    log_to_file(e);
                }
            }
            app.api_loading_kind = None;
        }
        ApiResponse::ReloadApiCLient(result) => match result {
            Ok(client_state) => {
                app.noti.notify(
                    NotifyType::Success,
                    "Reloaded Api Client successfully".to_string(),
                );
                app.client_state = client_state;
                app.client_state.save().ok();
                app.api_cmd_tx.send(ApiCmd::FetchLibraryData).ok();
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
    }
    if !skip_clear {
        app.api_loading_kind = None;
    }
}
