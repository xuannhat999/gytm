use data::api_client::{Browser, BrowserEngine, BrowserProfile, GeckoContainer};
use error::{YError, YResult, log_to_file};
use reqwest::cookie::Jar;
use std::path::{Path, PathBuf};

use crate::client::{
    chromium::{
        build_jar_sapisid_from_chromium_cookies, filter_exp_chromium_cookies,
        get_chromium_profiles_from_root, read_chromium_cookies,
    },
    gecko::{
        build_jar_from_gecko_cookies, filter_exp_gecko_cookies, get_gecko_profiles_from_ini,
        get_gecko_profiles_from_sqlite, read_gecko_cookies,
    },
};
mod chromium;
pub mod gecko;

pub fn get_profiles_from_browser(browser: &Browser) -> YResult<Vec<BrowserProfile>> {
    let root = get_root_path_from_browser(browser).unwrap_or_default();
    let engine = browser.engine();
    let profiles = match engine {
        BrowserEngine::Gecko => {
            let profiles = get_gecko_profiles_from_sqlite(&root)?;
            if !profiles.is_empty() {
                return Ok(profiles);
            }
            get_gecko_profiles_from_ini(&root)?
        }
        BrowserEngine::Chromium => get_chromium_profiles_from_root(&root)?,
    };
    Ok(profiles)
}

pub fn load_cookies(
    browser: &Browser,
    profile: &BrowserProfile,
    container: Option<&GeckoContainer>,
) -> YResult<Option<(Jar, String)>> {
    let engine = browser.engine();
    let db_path = get_db_path_from_profile_path(&profile.path, &engine)
        .ok_or_else(|| error::YError::InvalidPath("Selected browser's db file path".to_string()))?;
    let res = match engine {
        BrowserEngine::Gecko => {
            let cookies = read_gecko_cookies(
                &db_path,
                container
                    .ok_or(YError::MissApiClientContext("Container".to_string()))?
                    .id,
            )?;
            let exp_filtered_cookies = filter_exp_gecko_cookies(cookies);
            build_jar_from_gecko_cookies(exp_filtered_cookies)
        }
        BrowserEngine::Chromium => {
            let root_path = profile.path.parent().ok_or_else(|| {
                error::YError::InvalidPath("Selected browser's root dir".to_string())
            })?;
            let local_state = get_file_path_from_root_and_filename(root_path, "Local State")
                .ok_or_else(|| {
                    error::YError::InvalidPath("Selected browsser's Local State".to_string())
                })?;
            let cookies = read_chromium_cookies(&db_path, &local_state)?;
            let exp_filtered_cookies = filter_exp_chromium_cookies(cookies);
            build_jar_sapisid_from_chromium_cookies(exp_filtered_cookies)
        }
    };
    log_to_file(format!(
        "Loaded cookies from : BROWSER: {:?} | PROFILE: {:?} | CONTAINER: {:?}",
        browser, profile, container
    ));
    res
}

pub(super) fn get_db_path_from_profile_path(
    profile_path: &Path,
    browser_engine: &BrowserEngine,
) -> Option<PathBuf> {
    let cookie_filename = match browser_engine {
        BrowserEngine::Gecko => "cookies.sqlite",
        BrowserEngine::Chromium => "Cookies",
    };
    let cookie_filepath = profile_path.join(cookie_filename);
    if !cookie_filepath.exists() {
        return None;
    }
    Some(cookie_filepath)
}

pub(super) fn get_file_path_from_root_and_filename(root: &Path, filename: &str) -> Option<PathBuf> {
    let file_path = root.join(filename);
    if !file_path.exists() || !file_path.is_file() {
        return None;
    }
    Some(file_path)
}

fn get_root_path_from_browser(browser: &Browser) -> Option<PathBuf> {
    let config_dir = dirs::config_dir().unwrap_or_default();
    let home_dir = dirs::home_dir().unwrap_or_default();
    let roots = match browser {
        Browser::Chrome => vec![
            "chromium",
            "snap/chromium/common/chromium",
            ".var/app/org.chromium.Chromium/config/chromium",
        ],
        Browser::GoogleChrome => vec![
            "google-chrome",
            "google-chrome-beta",
            "google-chrome-unstable",
            "snap/google-chrome/common/.config/google-chrome",
            ".var/app/com.google.Chrome/config/google-chrome",
        ],
        Browser::Brave => vec![
            "BraveSoftware/Brave-Browser",
            "BraveSoftware/Brave-Browser-Beta",
            "BraveSoftware/Brave-Browser-Dev",
            "BraveSoftware/Brave-Browser-Nightly",
            "snap/brave/common/.config/BraveSoftware/Brave-Browser",
            ".var/app/com.brave.Browser/config/BraveSoftware/Brave-Browser",
        ],
        Browser::BraveOrigin => vec![
            "BraveSoftware/Brave-Origin",
            "BraveSoftware/Brave-Origin-Beta",
            "BraveSoftware/Brave-Origin-Nightly",
            "snap/brave/common/.config/BraveSoftware/Brave-Browser-Beta",
            ".var/app/com.brave.Browser/config/BraveSoftware/Brave-Browser-Nightly",
        ],
        Browser::Edge => vec![
            "microsoft-edge",
            "microsoft-edge-beta",
            "microsoft-edge-dev",
            ".var/app/com.microsoft.Edge/config/microsoft-edge",
        ],
        Browser::Vilvadi => vec![
            "vivaldi",
            "vivaldi-beta",
            "vivaldi-snapshot",
            ".var/app/com.vivaldi.Vivaldi/config/vivaldi",
        ],
        Browser::FireFox => vec![
            "mozilla/firefox",
            ".mozilla/firefox",
            ".mozilla/firefox-trunk",
            "snap/firefox/common/.mozilla/firefox",
            ".var/app/org.mozilla.firefox/.mozilla/firefox",
        ],
        Browser::LibreWolf => vec![
            "librewolf/librewolf",
            ".var/app/io.gitlab.LibreWolf-Community/.librewolf",
        ],
        Browser::Zen => vec!["zen", ".var/app/app.zen_browser.zen/data/zen"],
    };
    for p in roots {
        let temp_path = config_dir.join(p);
        if temp_path.exists() {
            return Some(temp_path);
        }
        let temp_path = home_dir.join(p);
        if temp_path.exists() {
            return Some(temp_path);
        }
    }
    None
}
