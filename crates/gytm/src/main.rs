use api::{
    YClient,
    protocol::{ApiCmd, ApiResponse},
};
use crossterm::{
    event::{self, Event},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use data::{MpvCommand, MpvEvent};
use error::{YResult, log_to_file};
use player::Player;
use ratatui::{Terminal, backend::CrosstermBackend};
use state::AppState;
use std::{io, sync::Arc, time::Duration};
use tokio::sync::mpsc;
use tui::{
    app::App,
    handler,
    helper::remove_queue_file,
    theme::Theme,
    ui::{self},
};

#[tokio::main]
async fn main() -> YResult<()> {
    let mut app_state = match AppState::load() {
        Ok(c) => c,
        Err(e) => {
            println!("{}", e);
            std::process::exit(1);
        }
    };
    let (api_cmd_tx, mut api_cmd_rx) = mpsc::unbounded_channel::<ApiCmd>();
    let (api_res_tx, mut api_res_rx) = mpsc::unbounded_channel::<ApiResponse>();
    let mut app = App::new(&app_state.player_state, api_cmd_tx);
    println!("󱘖 Connecting to YouTube Music...");
    let client = match YClient::new().await {
        Ok(c) => Arc::new(c),
        Err(e) => {
            log_to_file(&e);
            println!("{}", e);
            std::process::exit(1);
        }
    };
    let worker_client = client.clone();
    tokio::spawn(async move {
        while let Some(cmd) = api_cmd_rx.recv().await {
            let res = match cmd {
                ApiCmd::CreatePlaylist {
                    title,
                    description,
                    privacy,
                } => ApiResponse::CreatePlaylist(
                    worker_client
                        .create_playlist(&title, &description, privacy)
                        .await,
                ),
                ApiCmd::SaveSong { song, playlist_id } => ApiResponse::SaveSong(
                    match worker_client.save_to_playlist(&song, &playlist_id).await {
                        Ok(_) => Ok((song, playlist_id)),
                        Err(e) => Err(e),
                    },
                ),
                ApiCmd::Search(query) => {
                    let (albums, songs) = tokio::join!(
                        worker_client.get_search_albums(&query),
                        worker_client.get_search_songs(&query)
                    );
                    ApiResponse::Search { albums, songs }
                }
                ApiCmd::LikeSong(song) => {
                    ApiResponse::LikeSong(match worker_client.like_song(&song).await {
                        Ok(_) => Ok(song),
                        Err(e) => Err(e),
                    })
                }
                ApiCmd::UnlikeSong(song) => {
                    let res = match worker_client.unlike_song(&song).await {
                        Ok(_) => Ok(()),
                        Err(e) => Err(e),
                    };
                    ApiResponse::UnlikeSong((res, song.title))
                }
                ApiCmd::UnsaveSong { song, playlist_id } => {
                    let res = match worker_client.unsave_to_playlist(&song, &playlist_id).await {
                        Ok(_) => Ok(()),
                        Err(e) => Err(e),
                    };
                    ApiResponse::UnsaveSong((res, song.title))
                }
                ApiCmd::GetSongsToView(playlist) => {
                    let songs = worker_client.get_songs(&playlist.browse_id).await;
                    ApiResponse::GetSongsToView { songs, playlist }
                }
                ApiCmd::GetSongsToPlay(playlist) => {
                    let songs = worker_client.get_songs(&playlist.browse_id).await;
                    ApiResponse::GetSongsToPlay {
                        songs,
                        playlist_id: playlist.playlist_id,
                    }
                }
                ApiCmd::UnsaveAlbum(playlist) => {
                    let res = match worker_client.unsave_album_raw(&playlist.playlist_id).await {
                        Ok(_) => Ok(()),
                        Err(e) => Err(e),
                    };
                    ApiResponse::UnsaveAlbum((res, playlist))
                }
                ApiCmd::UnsaveCusPlaylist(playlist) => {
                    let res = match worker_client
                        .unsave_cus_playlist_raw(&playlist.playlist_id)
                        .await
                    {
                        Ok(_) => Ok(()),
                        Err(e) => Err(e),
                    };
                    ApiResponse::UnsaveCusPlaylist((res, playlist.title))
                }
                ApiCmd::SaveAlbum(album) => {
                    let res = match worker_client.save_album_raw(&album.playlist_id).await {
                        Ok(_) => Ok(()),
                        Err(e) => Err(e),
                    };
                    ApiResponse::SaveAlbum((res, album))
                }
                ApiCmd::GetRelatedSongsToPlay(song) => {
                    match worker_client.get_params(&song.video_id).await {
                        Ok(params) => {
                            let related_songs =
                                worker_client.get_related_songs(song, &params).await;
                            ApiResponse::GetRelatedSongsToPlay(related_songs)
                        }
                        Err(e) => ApiResponse::GetRelatedSongsToPlay(Err(e)),
                    }
                }
                ApiCmd::FetchLibraryData => {
                    ApiResponse::FetchLibraryData(worker_client.get_lists().await)
                }
            };
            if api_res_tx.send(res).is_err() {
                break;
            }
        }
    });
    app.api_cmd_tx.send(ApiCmd::FetchLibraryData).ok();
    app.api_loading_kind = Some(api::protocol::ApiLoadingKind::FetchLibraryData);
    let mut player = Player::default();
    let (tx_event, mut rx) = mpsc::channel::<MpvEvent>(32);

    if Player::check_socket_exists().is_ok()
        && let Ok(stream) = player.connect_mpv().await
    {
        let _ = app.load_queue_file();
        player.observe_mpv(stream, tx_event).await?;
    } else {
        remove_queue_file();
        player.spawn_mpv()?;
        let stream = player.connect_mpv().await?;
        player.observe_mpv(stream, tx_event).await?;
        player.send_mpv_command(MpvCommand::SetVol(app.volume))?;
    }
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    app.albums_liststate.select(Some(0));
    app.playlists_liststate.select(Some(0));

    let theme = Theme::default();

    let mut render = true;
    let mut last_tick = std::time::Instant::now();
    let start_time = std::time::Instant::now();
    loop {
        let had_notification = app.noti.has_notification();
        let elapsed = std::cmp::min(last_tick.elapsed(), Duration::from_millis(100));
        last_tick = std::time::Instant::now();
        app.noti.tick(elapsed);
        while let Ok(event) = rx.try_recv() {
            handler::handle_mpv_event(&mut app, &mut app_state, event);
            render = true;
        }
        while let Ok(response) = api_res_rx.try_recv() {
            handler::handle_api_response(&mut app, response, &player);
            render = true;
        }
        if event::poll(Duration::from_millis(50))? {
            match event::read()? {
                Event::Key(key) => {
                    handler::handle_key_events(key, &mut app, &mut player, &mut app_state);
                    render = true;
                }
                Event::Resize(_, _) => {
                    render = true;
                }
                _ => {}
            }
        }
        if app.is_exit {
            break;
        }

        if app.noti.has_notification() || had_notification {
            render = true;
        }
        if app.api_loading_kind.is_some() {
            render = true;
        }
        if render {
            terminal.draw(|f| {
                ui::render(&mut app, f, &theme, start_time);
                app.noti.render(f, f.area());
            })?;
            render = false;
        }
    }
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}
