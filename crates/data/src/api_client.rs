use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    Vivaldi,
    // Gecko
    FireFox,
    LibreWolf,
    Zen,
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
    Browser::Vivaldi,
    Browser::FireFox,
    Browser::LibreWolf,
    Browser::Zen,
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserProfile {
    pub name: String,
    pub path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeckoContainer {
    pub name: String,
    pub id: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Account {
    pub email: String,
    pub auth_user: usize,
}
