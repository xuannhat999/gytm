use api::YClient;
use crossterm::{
    event::{self, Event},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use data::MpvEvent;
use error::log_to_file;
use player::Player;
use ratatui::{Terminal, backend::CrosstermBackend};
use state::AppState;
use std::{io, sync::Arc, thread, time::Duration};
use tui::{app::App, handler, ui};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut state = match AppState::load() {
        Ok(c) => c,
        Err(e) => {
            println!("{}", e);
            std::process::exit(1);
        }
    };
    println!("󱎫 Connecting to YouTube Music...");
    let client = match YClient::new(&state).await {
        Ok(c) => Arc::new(c),
        Err(e) => {
            log_to_file(&e);
            println!("{}", e);
            std::process::exit(1);
        }
    };
    let (albums, playlists) = client.get_lists().await?;

    let mut app = App::default();
    let mut player = Player::new(&state.player_state);
    app.albums = albums;
    app.playlists = playlists;

    let (tx, rx) = std::sync::mpsc::channel::<MpvEvent>();

    if let Err(e) = player.start_mpv() {
        log_to_file(&e);
        println!("Error starting MPV: {}", e);
        std::process::exit(1);
    }

    thread::sleep(Duration::from_millis(300));

    if let Err(e) = player.observe_mpv_changes(tx).await {
        log_to_file(&e);
        println!("{}", e);
        std::process::exit(1);
    }

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    if !app.albums.is_empty() {
        app.album_list_state.select(Some(0));
    } else {
        app.playlist_list_state.select(Some(0));
    }

    loop {
        while let Ok(event) = rx.try_recv() {
            handler::handle_mpv_event(&mut app, &mut player, &mut state, event);
        }
        if event::poll(Duration::from_millis(750))? {
            if let Event::Key(key) = event::read()? {
                handler::handle_key_events(
                    key,
                    &mut app,
                    Arc::clone(&client),
                    &mut player,
                    &mut state,
                )
                .await;
            }
        }

        terminal.draw(|f| ui::render(&mut app, f, &player))?;
        if app.is_exit {
            player.kill_current_process();
            break;
        }
    }
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}
