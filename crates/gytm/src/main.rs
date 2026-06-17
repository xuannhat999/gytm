use api::YClient;
use crossterm::{
    event::{self, Event},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use data::{MpvCommand, MpvEvent, file_path::MPV_PLAYLIST};
use error::{YResult, log_to_file};
use player::Player;
use ratatui::{Terminal, backend::CrosstermBackend};
use state::AppState;
use std::{fs, io, sync::Arc, time::Duration};
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
    let mut state = match AppState::load() {
        Ok(c) => c,
        Err(e) => {
            println!("{}", e);
            std::process::exit(1);
        }
    };
    let mut app = App::new(&state.player_state);
    println!("󱘖 Connecting to YouTube Music...");
    let client = match YClient::new().await {
        Ok(c) => Arc::new(c),
        Err(e) => {
            log_to_file(&e);
            println!("{}", e);
            std::process::exit(1);
        }
    };
    println!(" Fetching data from Youtube Music...");
    app.fetch_data(&client).await?;
    let mut player = Player::default();
    let (tx_event, mut rx) = mpsc::channel::<MpvEvent>(32);
    if player.reconnect(tx_event.clone()).await.is_err() {
        remove_queue_file();
        player.spawn_mpv()?;
        player.connect_observe_mpv(tx_event).await?;
    } else {
        let _ = app.load_queue_file();
    }
    player.send_mpv_command(MpvCommand::SetVol(app.volume))?;
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
    loop {
        let had_notification = app.noti.has_notification();
        let elapsed = std::cmp::min(last_tick.elapsed(), Duration::from_millis(100));
        last_tick = std::time::Instant::now();
        app.noti.tick(elapsed);
        while let Ok(event) = rx.try_recv() {
            handler::handle_mpv_event(&mut app, &mut state, event);
            render = true;
        }
        if event::poll(Duration::from_millis(50))? {
            match event::read()? {
                Event::Key(key) => {
                    handler::handle_key_events(key, &mut app, &client, &mut player, &mut state)
                        .await;
                    render = true;
                }
                Event::Resize(_, _) => {
                    render = true;
                }
                _ => {}
            }
        }
        if app.is_exit {
            let _ = fs::remove_file(MPV_PLAYLIST);
            break;
        }

        if app.noti.has_notification() || had_notification {
            render = true;
        }
        if render {
            terminal.draw(|f| {
                ui::render(&mut app, f, &theme);
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
