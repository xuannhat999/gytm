use std::sync::Arc;

use api::YClient;
use error::{YResult, log_to_file};
use player::Player;
use state::AppState;
use tui::app::App;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let state = match AppState::load() {
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
    // let (albums, playlists) = client.get_lists().await?;
    //
    // let mut app = App::default();
    // app.albums = albums;
    // app.playlists = playlists;
    //
    let res = client
        .remove_from_lib("OLAK5uy_nSFpJd6fk5g2u7CcljXZCqauq_CHCoP58")
        .await?;
    println!("{}", res);
    Ok(())
}
