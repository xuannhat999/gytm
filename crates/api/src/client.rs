use std::{
    fs,
    path::{Path, PathBuf},
};

use data::client::{Browser, BrowserEngine, BrowserProfile, GeckoContainer};
use error::{YError, YResult};
use ini::Ini;
use rookie::enums::Cookie;
use rusqlite::Connection;

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

fn get_db_path_from_profile_path(
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

fn read_cookies_by_container_id(
    db_path: &Path,
    container_id: Option<i32>,
) -> YResult<Vec<GeckoCookie>> {
    let tmp_path = copy_sqlite_with_wal(db_path)?;

    let result = (|| -> YResult<Vec<GeckoCookie>> {
        let conn = Connection::open(&tmp_path)?;
        let domain = ".youtube.com";
        let host_pattern = format!("%{}%", domain);
        match container_id {
            Some(container_id) => {
                let exact_pattern = format!("^userContextId={}", container_id);
                let mut stmt = conn.prepare(
                    "SELECT name, value, host, path, expiry, isSecure, isHttpOnly, originAttributes
             FROM moz_cookies
             WHERE host LIKE ?1
               AND (originAttributes = ?2 OR originAttributes LIKE ?3)",
                )?;

                let prefix_pattern = format!("^userContextId={}&%", container_id); // trường hợp có thêm &privateBrowsingId... phía sau

                let rows = stmt.query_map(
                    rusqlite::params![host_pattern, exact_pattern, prefix_pattern],
                    |row| {
                        Ok(GeckoCookie {
                            name: row.get(0)?,
                            value: row.get(1)?,
                            host: row.get(2)?,
                            path: row.get(3)?,
                            expiry: row.get(4)?,
                            is_secure: row.get::<_, i64>(5)? != 0,
                            is_http_only: row.get::<_, i64>(6)? != 0,
                            origin_attributes: row.get(7)?,
                        })
                    },
                )?;
                rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
            }
            None => {
                let mut stmt = conn.prepare(
                    "SELECT name, value, host, path, expiry, isSecure, isHttpOnly, originAttributes
             FROM moz_cookies
             WHERE host LIKE ?1
               AND originAttributes = ''",
                )?;
                let rows = stmt.query_map(rusqlite::params![host_pattern], |row| {
                    Ok(GeckoCookie {
                        name: row.get(0)?,
                        value: row.get(1)?,
                        host: row.get(2)?,
                        path: row.get(3)?,
                        expiry: row.get(4)?,
                        is_secure: row.get::<_, i64>(5)? != 0,
                        is_http_only: row.get::<_, i64>(6)? != 0,
                        origin_attributes: row.get(7)?,
                    })
                })?;
                rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
            }
        }
    })();
    cleanup_temp_sqlite(&tmp_path);
    result
}

pub fn get_geckgo_containers_from_profile(profile_path: &Path) -> YResult<Vec<GeckoContainer>> {
    let file_path = get_file_path_from_root_and_filename(profile_path, "containers.json");
    if let Some(file) = file_path {
        return get_gecko_containers_from_file(&file);
    }
    Ok(Vec::new())
}

fn get_gecko_containers_from_file(file_path: &Path) -> YResult<Vec<GeckoContainer>> {
    let content = fs::read_to_string(file_path)?;
    let json = gjson::get(&content, "identities");
    let mut containers = vec![GeckoContainer {
        id: None,
        name: "No container".to_string(),
    }];
    json.each(|_, val| {
        let is_public = val.get("public").bool();
        if is_public {
            let name = val.get("name");
            let l10nId = val.get("l10nId");
            let id = val.get("userContextId").i32();
            let container_name = if name.exists() {
                name.str().to_string()
            } else {
                l10nId.str().to_string()
            };
            containers.push(GeckoContainer {
                id: Some(id),
                name: container_name.to_string(),
            });
        }
        true
    });
    Ok(containers)
}

fn get_cookie_from_db_chromium(db_path: &Path, key_path: &Path) -> YResult<Vec<Cookie>> {
    let domains = vec![".youtube.com".to_string()];
    let str_db_path = db_path.to_string_lossy().into_owned();
    let str_key_path = key_path.to_string_lossy().into_owned();
    let cookies = rookie::any_browser(&str_db_path, Some(domains), Some(&str_key_path));
    match cookies {
        Ok(c) => Ok(c),
        Err(e) => Err(YError::RookieError(e.to_string())),
    }
}

fn get_file_path_from_root_and_filename(root: &Path, filename: &str) -> Option<PathBuf> {
    let file_path = root.join(filename);
    if !file_path.exists() || !file_path.is_file() {
        return None;
    }
    Some(file_path)
}

fn get_chromium_profiles_from_root(root: &Path) -> YResult<Vec<BrowserProfile>> {
    let local_state_path = get_file_path_from_root_and_filename(root, "Local State");
    if let Some(local_state) = local_state_path {
        let content = fs::read_to_string(&local_state)?;
        let json = gjson::get(&content, "profile.info_cache");
        if !json.exists() {
            return Err(YError::InvalidFileContent(local_state));
        }
        let mut profiles: Vec<BrowserProfile> = Vec::new();
        json.each(|key, value| {
            let name = value.get("name");
            if name.exists() {
                let path = root.join(key.to_string());
                if path.exists() && path.is_dir() {
                    profiles.push(BrowserProfile {
                        name: name.str().to_string(),
                        path,
                    });
                }
            } else {
                return false;
            }
            true
        });
        return Ok(profiles);
    }
    Ok(Vec::new())
}

fn get_gecko_profiles_from_sqlite(root_path: &Path) -> YResult<Vec<BrowserProfile>> {
    let profile_groups_dir = root_path.join("Profile Groups");
    if !profile_groups_dir.exists() {
        return Err(YError::InvalidPath(
            profile_groups_dir.to_string_lossy().into_owned(),
        ));
    }

    let sqlite_path = fs::read_dir(&profile_groups_dir)?
        .filter_map(|e| e.ok())
        .find(|e| {
            let p = e.path();
            p.extension().is_some_and(|ext| ext == "sqlite")
        })
        .map(|e| e.path());

    let sqlite_path = match sqlite_path {
        Some(p) => p,
        None => return Ok(Vec::new()),
    };

    let tmp_sqlite = copy_sqlite_with_wal(&sqlite_path)?;
    let conn = Connection::open(&tmp_sqlite)?;

    let profiles = conn
        .prepare("SELECT path, name FROM Profiles")
        .and_then(|mut stmt| {
            stmt.query_map([], |row| {
                let path: String = row.get(0)?;
                let name: String = row.get(1)?;
                Ok(BrowserProfile {
                    name,
                    path: root_path.join(path),
                })
            })?
            .collect::<Result<Vec<_>, _>>()
        })?;
    cleanup_temp_sqlite(&tmp_sqlite);
    Ok(profiles)
}

fn get_gecko_profiles_from_ini(root_path: &Path) -> YResult<Vec<BrowserProfile>> {
    let ini_path = root_path.join("profiles.ini");
    if !ini_path.exists() || !ini_path.is_file() {
        return Err(YError::InvalidPath(ini_path.to_string_lossy().into_owned()));
    }
    let ini = Ini::load_from_file(&ini_path)?;
    let mut profiles = Vec::new();
    for section_name in ini.sections().flatten() {
        if !section_name.starts_with("Profile") {
            continue;
        }
        let properties = match ini.section(Some(section_name)) {
            Some(p) => p,
            None => continue,
        };
        let name: &str = properties.get("Name").map_or("", |n| n);
        let path_str: &str = properties.get("Path").map_or("", |p| p);
        let is_relative = properties.get("IsRelative").map_or(false, |v| v == "1");
        if name.is_empty() || path_str.is_empty() {
            continue;
        }
        let full_path = if is_relative {
            root_path.join(path_str)
        } else {
            PathBuf::from(path_str)
        };
        if full_path.exists() && full_path.is_dir() {
            profiles.push(BrowserProfile {
                name: name.to_string(),
                path: full_path,
            });
        }
    }
    Ok(profiles)
}

fn copy_sqlite_with_wal(db_path: &Path) -> YResult<PathBuf> {
    let tmp_dir = std::env::temp_dir();
    let unique_name = format!("gecko_cookies_{}.sqlite", std::process::id());
    let tmp_path = tmp_dir.join(unique_name);

    fs::copy(db_path, &tmp_path)?;

    for ext in ["-wal", "-shm"] {
        let src = PathBuf::from(format!("{}{}", db_path.display(), ext));
        if src.exists() {
            let dst = PathBuf::from(format!("{}{}", tmp_path.display(), ext));
            let _ = fs::copy(&src, &dst); // best-effort
        }
    }
    Ok(tmp_path)
}

fn cleanup_temp_sqlite(tmp_path: &Path) {
    let _ = fs::remove_file(tmp_path);
    let _ = fs::remove_file(format!("{}-wal", tmp_path.display()));
    let _ = fs::remove_file(format!("{}-shm", tmp_path.display()));
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeckoCookie {
    pub(crate) name: String,
    pub(crate) value: String,
    pub(crate) host: String,
    pub(crate) path: String,
    pub(crate) expiry: i64,
    pub(crate) is_secure: bool,
    pub(crate) is_http_only: bool,
    pub(crate) origin_attributes: String,
}
