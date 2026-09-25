use super::{append_song_to_queue, clear_queue, load_list, remove_song_from_queue};
use crate::{app::App, handler::table_event::handle_table_event, notification::NotifyType};
use api::{
    client::{self, gecko::get_gecko_containers_from_profile, get_profiles_from_browser},
    protocol::{ApiCmd, ApiLoadingKind},
};
use config::Config;
use crossterm::event::{KeyCode, KeyEvent};
use data::{
    api_client::{ALL_BROWSERS, BrowserEngine},
    app::{
        AppPage, CreatePlaylistFocus,
        FocusArea::{self},
        PlayListPrivacy, PlayMode, PopupState, SearchSongSource,
    },
    mpv::MpvCommand,
};
use error::log_to_file;
use player::Player;
use ratatui::widgets::ListState;
use state::{Persist, client_state::ClientState};

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
                KeyCode::Char('i') => {
                    app.popup_state = PopupState::ApiCLient;
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
                _ => match app.focus_area {
                    FocusArea::Albums => handle_albums_key(app, key_event.code),
                    FocusArea::Playlists => handle_playlists_key(app, key_event.code),
                    _ => {}
                },
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
                        _ => match app.focus_area {
                            FocusArea::SearchAlbums => match key_event.code {
                                KeyCode::Char('x') => {
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
                                KeyCode::Enter => {
                                    let selected = app
                                        .search_albums_liststate
                                        .selected()
                                        .map(|i| &app.search_albums[i]);
                                    if let Some(album) = selected {
                                        app.api_cmd_tx
                                            .send(ApiCmd::GetSongsToPlay(album.clone()))
                                            .ok();
                                        app.api_loading_kind = Some(ApiLoadingKind::GetSongsToPlay);
                                        app.focus_area = FocusArea::Queue;
                                    }
                                }
                                KeyCode::Char('l') => {
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
                                _ => {}
                            },
                            FocusArea::SearchSongs => match key_event.code {
                                KeyCode::Char('x') => {
                                    if let Some(song) = app.selected_search_song() {
                                        app.popup_state = PopupState::SaveSong {
                                            selected_save_song: song.clone(),
                                        };
                                        app.cus_playlists_liststate.select(Some(0));
                                    }
                                }
                                KeyCode::Char('a') => {
                                    if let Some(song) = app.selected_search_song() {
                                        if let Err(e) =
                                            append_song_to_queue(app, player, song.clone())
                                        {
                                            log_to_file(&e);
                                        }
                                    }
                                }
                                KeyCode::Enter => {
                                    let selected = app.selected_search_song();
                                    if let Some(song) = selected {
                                        app.api_cmd_tx
                                            .send(ApiCmd::GetRelatedSongsToPlay(song.clone()))
                                            .ok();
                                        app.api_loading_kind = Some(ApiLoadingKind::GetSongsToPlay);
                                        app.focus_area = FocusArea::Queue;
                                    }
                                }
                                KeyCode::Char('l') | KeyCode::Char('h') => {
                                    app.toggle_search_songs_source();
                                }
                                _ => {}
                            },
                            _ => {}
                        },
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
            FocusArea::SearchSongs => match app.search_songs_source {
                SearchSongSource::Song => (&mut app.search_songs_liststate, app.search_songs.len()),
                SearchSongSource::Video => {
                    (&mut app.search_videos_liststate, app.search_videos.len())
                }
            },
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
        KeyCode::Char(' ') if app.playing_song_idx.is_some() => {
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
            KeyCode::Esc => app.popup_state = PopupState::ApiCLient,
            KeyCode::Char('h') | KeyCode::Left => app.popup_state = PopupState::ApiCLient,
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
            KeyCode::Esc => app.popup_state = PopupState::ApiCLient,
            KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
                if let Some(index) = profiles_liststate.selected() {
                    let profile = &profiles[index];
                    match browser.engine() {
                        BrowserEngine::Gecko => {
                            if let Ok(containers) = get_gecko_containers_from_profile(&profile.path)
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
            KeyCode::Esc => app.popup_state = PopupState::ApiCLient,
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
            KeyCode::Esc => {
                app.api_cmd_tx.send(ApiCmd::DiscardPendingClient).ok();
                app.popup_state = PopupState::ApiCLient;
            }
            KeyCode::Char('h') | KeyCode::Left => {
                app.api_cmd_tx.send(ApiCmd::DiscardPendingClient).ok();
                if container.is_some() {
                    match get_gecko_containers_from_profile(&profile.path) {
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
                    app.api_cmd_tx.send(ApiCmd::SetClient(client_state)).ok();
                    app.api_loading_kind = Some(ApiLoadingKind::ReloadClient);
                    if !matches!(app.popup_state, PopupState::None) {
                        app.popup_state = PopupState::ApiCLient;
                    }
                }
            }
            _ => {}
        },
        PopupState::ApiCLient => match key_event.code {
            KeyCode::Esc => {
                app.popup_state = PopupState::None;
            }
            KeyCode::Char('b') => {
                app.popup_state = PopupState::SelectBrowser;
                app.browser_liststate.select(Some(0));
            }
            KeyCode::Char('p') => {
                if let Ok((browser, _, _, _)) = app.client_state.get_validated_fields() {
                    match get_profiles_from_browser(browser) {
                        Ok(profiles) => {
                            let mut profiles_liststate = ListState::default();
                            if !profiles.is_empty() {
                                profiles_liststate.select(Some(0));
                            }
                            app.popup_state = PopupState::SelectBrowserProfile {
                                browser: *browser,
                                profiles,
                                profiles_liststate,
                            };
                        }
                        Err(e) => log_to_file(e),
                    }
                }
            }
            KeyCode::Char('c') => {
                if let Ok((browser, profile, _, _)) = app.client_state.get_validated_fields()
                    && browser.engine() == BrowserEngine::Gecko
                    && let Ok(containers) = get_gecko_containers_from_profile(&profile.path)
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
            KeyCode::Char('a') => match app.client_state.get_validated_fields() {
                Ok((browser, profile, container, _)) => {
                    let client = ClientState {
                        browser: Some(*browser),
                        profile: Some(profile.clone()),
                        gecko_container: container.cloned(),
                        account: None,
                    };
                    app.api_loading_kind = Some(ApiLoadingKind::FetchAccountsList);
                    app.api_cmd_tx.send(ApiCmd::FetchAccountsList(client)).ok();
                }
                Err(_) => {
                    app.popup_state = PopupState::ApiCLient;
                }
            },
            KeyCode::Char('r') => {
                if app.client_state.get_validated_fields().is_ok() {
                    app.api_cmd_tx
                        .send(ApiCmd::ReloadApiClient(app.client_state.clone()))
                        .ok();
                    app.api_loading_kind = Some(ApiLoadingKind::ReloadClient);
                } else {
                    app.noti.notify(
                        NotifyType::Error,
                        String::from("YTM client not configured, press b to setup"),
                    );
                }
            }
            KeyCode::Char('g') => {
                app.api_cmd_tx.send(ApiCmd::ToggleGuest()).ok();
                app.client_state = ClientState::default();
                app.client_state.save().ok();
                app.albums.clear();
                app.playlists.clear();
            }

            _ => {}
        },
        _ => {}
    }
}

fn handle_albums_key(app: &mut App, key_code: KeyCode) {
    if handle_table_event(&mut app.albums_tablestate, key_code) {
        return;
    }
    match key_code {
        KeyCode::Char('l') => view_list_content(app, FocusArea::Albums),
        KeyCode::Char('x') => unsave_album(app),
        KeyCode::Enter => play_list(app, FocusArea::Albums),
        _ => {}
    }
}
fn handle_playlists_key(app: &mut App, key_code: KeyCode) {
    if handle_table_event(&mut app.playlists_tablestate, key_code) {
        return;
    }
    match key_code {
        KeyCode::Char('l') => view_list_content(app, FocusArea::Playlists),
        KeyCode::Char('x') => unsave_playlist(app),
        KeyCode::Char('a') => {
            app.popup_state = PopupState::CreatePlaylist {
                title: String::new(),
                description: String::new(),
                privacy: PlayListPrivacy::Private,
                focused_field: CreatePlaylistFocus::Title,
            };
        }
        KeyCode::Enter => play_list(app, FocusArea::Playlists),
        _ => {}
    }
}
fn unsave_album(app: &mut App) {
    if let Some(i) = app.albums_tablestate.selected() {
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

fn unsave_playlist(app: &mut App) {
    if let Some(i) = app.playlists_tablestate.selected() {
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
fn view_list_content(app: &mut App, focus_area: FocusArea) {
    let list = match focus_area {
        FocusArea::Albums => app
            .albums_tablestate
            .selected()
            .and_then(|i| app.albums.get(i)),
        FocusArea::Playlists => app
            .playlists_tablestate
            .selected()
            .and_then(|i| app.playlists.get(i)),
        _ => None,
    };
    if let Some(list) = list {
        app.api_cmd_tx
            .send(ApiCmd::GetSongsToView(list.clone()))
            .ok();
        app.focus_area = FocusArea::Songs;
        app.api_loading_kind = Some(ApiLoadingKind::GetSongsToView);
    }
}

fn play_list(app: &mut App, focus_area: FocusArea) {
    let list = match focus_area {
        FocusArea::Albums => app
            .albums_tablestate
            .selected()
            .and_then(|i| app.albums.get(i)),
        FocusArea::Playlists => app
            .playlists_tablestate
            .selected()
            .and_then(|i| app.playlists.get(i)),
        _ => None,
    };
    if let Some(list) = list {
        app.api_cmd_tx
            .send(ApiCmd::GetSongsToPlay(list.clone()))
            .ok();
        app.api_loading_kind = Some(ApiLoadingKind::GetSongsToPlay);
        app.focus_area = FocusArea::Queue;
    }
}
