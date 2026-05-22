use api::YClient;
use data::AppConfig;
use std::sync::Arc;

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
    let (albums, songs) = client.get_lists().await?;
    Ok(())
}
