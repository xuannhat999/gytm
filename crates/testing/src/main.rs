use data::Song;
use player::MpvEvent;
use player::Player;
use std::{thread, time::Duration};
use tui::handler;
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut player = Player::default();
    let mut app = tui::app::App::default();
    let (tx, rx) = std::sync::mpsc::channel::<MpvEvent>();
    player.start_mpv();
    thread::sleep(Duration::from_millis(50));
    player.listen_playlist_changes(tx).await;
    println!("Listening event from mpv...");

    let mut songs: Vec<Song> = Vec::default();
    let mut song1 = Song::default();
    song1.video_id = "a0JhydsuocI".to_string();
    song1.title = "あいつのブラウンシューズ - Aitsu no Brown Shoes".to_string();
    songs.push(song1);
    let mut song2 = Song::default();
    song2.video_id = "PhqavibeFzE".to_string();
    song2.title = "It's so Creamy".to_string();
    songs.push(song2);

    let mut song3 = Song::default();
    song3.video_id = "jgN7ZG36YGo".to_string();
    song3.title = "Jazzy Night".to_string();
    songs.push(song3);

    player.load_playlist(&songs).await;

    loop {
        while let Ok(event) = rx.try_recv() {
            handler::handle_mpv_event(&mut app, &mut player, event);
        }

        thread::sleep(Duration::from_secs(1));
    }
}
