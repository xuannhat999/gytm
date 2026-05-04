use player::MpvEvent;
use player::Player;
use std::{thread, time::Duration};
use tui::handler;
fn main() {
    let mut player = Player::default();
    let mut app = tui::app::App::default();
    println!("Listening event from mpv...");

    let (tx, rx) = std::sync::mpsc::channel::<MpvEvent>();
    player.start_mpv();
    thread::sleep(Duration::from_millis(100));
    player.listen_playlist_changes(tx);
    thread::sleep(Duration::from_secs(1));

    player.load_song("a0JhydsuocI", false);
    thread::sleep(Duration::from_secs(1));

    player.load_song("PhqavibeFzE", true);
    thread::sleep(Duration::from_secs(5));
    player.toggle_pause();
    loop {
        while let Ok(event) = rx.try_recv() {
            handler::handle_mpv_event(&mut app, &mut player, event);
        }

        thread::sleep(Duration::from_secs(1));
    }
}
