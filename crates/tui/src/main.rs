use api::YClient;
use crossterm::{
    event::{self, Event},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use data::AppConfig;
use ratatui::{Terminal, backend::CrosstermBackend};
use std::{io, sync::Arc};
mod app;
mod handler;
mod ui;

use crate::app::App;
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new();
    let config = match AppConfig::load() {
        Ok(c) => c,
        Err(e) => {
            println!("{}", e);
            std::process::exit(1);
        }
    };
    let client = match YClient::new(config).await {
        Ok(c) => Arc::new(c),
        Err(e) => {
            println!("{}", e);
            std::process::exit(1);
        }
    };
    let raw_data = client.get_lib_data().await?;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let (albums, playlists) = data::extract_albums(&raw_data);
    app.albums = albums;
    app.playlists = playlists;
    if !app.albums.is_empty() {
        app.album_list_state.select(Some(0));
    }
    loop {
        terminal.draw(|f| ui::render(&mut app, f))?;
        if let Event::Key(key) = event::read()? {
            handler::handle_key_events(key, &mut app, Arc::clone(&client)).await;
        }
        if app.is_exit {
            app.player.kill_current_process();
            break;
        }
    }
    // --- 4. KHÔI PHỤC TERMINAL (CLEANUP) ---
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
