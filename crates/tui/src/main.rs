use api::YClient;
use data::AppConfig;
use error::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let config = AppConfig::load().unwrap();
    let client = YClient::new(config).await?;
    let data = client.get_lib_data().await?;
    let play_lists = data::extract_albums(&data);
    for p in play_lists {
        println!("{}", p.title);
    }
    Ok(())
}
