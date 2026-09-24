pub mod bus;
pub mod client;
pub mod dao;
pub mod parser;
pub mod protocol;
pub mod request;
pub use bus::YTBus;
pub use dao::YTDao;

pub(crate) const YTM_URL: &str = "https://music.youtube.com";
pub(crate) const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";
pub(crate) const YTM_DOMAIN: &str = ".youtube.com";
