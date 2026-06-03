use api::YClient;
use crossterm::{
    event::{self, Event},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use data::{MpvEvent, Theme};
use error::log_to_file;
use player::Player;
use ratatui::{Terminal, backend::CrosstermBackend};
use state::AppState;
use std::{io, sync::Arc, time::Duration};
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
    let (albums, playlists) = client.get_lists().await?;

    let mut app = App::default();
    let mut player = Player::new(&state.player_state);
    app.albums = albums;
    app.playlists = playlists;

    let (tx, mut rx) = tokio::sync::mpsc::channel::<MpvEvent>(32);

    if let Err(e) = player.start_mpv_and_observe(tx) {
        log_to_file(&e);
        println!("Error starting MPV: {}", e);
        std::process::exit(1);
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
    loop {
        let had_notification = app.noti.has_notification();
        let elapsed = last_tick.elapsed();
        last_tick = std::time::Instant::now();
        app.noti.tick(elapsed);
        while let Ok(event) = rx.try_recv() {
            handler::handle_mpv_event(&mut app, &mut player, &mut state, event);
            render = true;
        }
        if event::poll(Duration::from_millis(50))? {
            match event::read()? {
                Event::Key(key) => {
                    handler::handle_key_events(
                        key,
                        &mut app,
                        Arc::clone(&client),
                        &mut player,
                        &mut state,
                    )
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
            player.kill_current_process();
            break;
        }
        if app.noti.has_notification() || had_notification {
            render = true;
        }
        if render {
            terminal.draw(|f| {
                ui::render(&mut app, f, &player, &theme);
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
