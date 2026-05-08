use api::YClient;
use crossterm::{
    event::{self, Event},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use data::AppConfig;
use player::{MpvEvent, Player};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::{io, sync::Arc, thread, time::Duration};
use tui::{app::App, handler, ui};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = match AppConfig::load() {
        Ok(c) => c,
        Err(e) => {
            println!("{}", e);
            std::process::exit(1);
        }
    };
    println!("󱎫 Connecting to YouTube Music...");
    let client = match YClient::new(config).await {
        Ok(c) => Arc::new(c),
        Err(e) => {
            println!("{}", e);
            std::process::exit(1);
        }
    };
    let raw_data = client.get_lib_data().await?;
    let (albums, playlists) = data::extract_lists(raw_data);
    if albums.is_empty() && playlists.is_empty() {
        println!("Error: No album/playlist found. Please check your config and cookies.");
        std::process::exit(1);
    }
    let mut app = App::default();
    let mut player = Player::default();
    app.albums = albums;
    app.playlists = playlists;

    let (tx, rx) = std::sync::mpsc::channel::<MpvEvent>();

    if let Err(e) = player.start_mpv() {
        println!("Error starting MPV: {}", e);
        std::process::exit(1);
    }

    thread::sleep(Duration::from_millis(200));

    if let Err(e) = player.listen_playlist_changes(tx).await {
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
    }
    loop {
        while let Ok(event) = rx.try_recv() {
            handler::handle_mpv_event(&mut app, &mut player, event);
        }
        if event::poll(Duration::from_millis(500))? {
            if let Event::Key(key) = event::read()? {
                handler::handle_key_events(key, &mut app, Arc::clone(&client), &mut player).await;
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
