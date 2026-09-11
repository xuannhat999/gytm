use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum BrowserEngine {
    Gecko,
    Chromium,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum Browser {
    // Chromium
    Chrome,
    GoogleChrome,
    Brave,
    BraveOrigin,
    Edge,
    Vilvadi,
    // Gecko
    FireFox,
    LibreWolf,
    Zen,
}
pub struct BrowserProfile {
    pub name: String,
    pub path: PathBuf,
}
impl Browser {
    pub const fn engine(&self) -> BrowserEngine {
        match self {
            Browser::LibreWolf | Browser::FireFox | Browser::Zen => BrowserEngine::Gecko,
            _ => BrowserEngine::Chromium,
        }
    }
}

pub static ALL_BROWSERS: &[Browser] = &[
    Browser::Chrome,
    Browser::GoogleChrome,
    Browser::Brave,
    Browser::BraveOrigin,
    Browser::Edge,
    Browser::Vilvadi,
    Browser::FireFox,
    Browser::LibreWolf,
    Browser::Zen,
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeckoContainer {
    pub name: String,
    pub id: Option<i32>,
}
