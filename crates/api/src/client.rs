use std::{
    fs,
    path::{Path, PathBuf},
};

use data::client::{Browser, BrowserEngine, BrowserProfile};
use error::{YError, YResult};
use ini::Ini;
use rusqlite::Connection;

pub fn get_profiles_from_browser(browser: &Browser) -> YResult<Vec<BrowserProfile>> {
    let root = get_root_path_from_unsupported_browser(browser).unwrap_or_default();
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

fn get_chromium_profiles_from_root(root: &Path) -> YResult<Vec<BrowserProfile>> {
    let target_filename = "Local State";
    let target_filepath = root.join(target_filename);
    if target_filepath.exists() && target_filepath.is_file() {
        let content = fs::read_to_string(target_filepath)?;
        let json = gjson::get(&content, "profile.info_cache");
        if !json.exists() {
            return Ok(Vec::new());
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
    Err(YError::InvalidPath(
        target_filepath.to_string_lossy().into_owned(),
    ))
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

    let stem = sqlite_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("profile_groups");

    let temp_dir = std::env::temp_dir();
    let tmp_sqlite = temp_dir.join(format!("{stem}.sqlite"));
    let tmp_shm = temp_dir.join(format!("{stem}.sqlite-shm"));
    let tmp_wal = temp_dir.join(format!("{stem}.sqlite-wal"));

    fs::copy(&sqlite_path, &tmp_sqlite)?;
    let _ = fs::copy(
        profile_groups_dir.join(format!("{stem}.sqlite-shm")),
        &tmp_shm,
    );
    let _ = fs::copy(
        profile_groups_dir.join(format!("{stem}.sqlite-wal")),
        &tmp_wal,
    );

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

    let _ = fs::remove_file(&tmp_sqlite);
    let _ = fs::remove_file(&tmp_shm);
    let _ = fs::remove_file(&tmp_wal);

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

fn get_root_path_from_unsupported_browser(browser: &Browser) -> Option<PathBuf> {
    let config_dir = dirs::config_dir().unwrap_or_default();
    let home_dir = dirs::home_dir().unwrap_or_default();
    let roots = match browser {
        Browser::BraveOrigin => vec!["BraveSoftware/Brave-Origin"],
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
        _ => Vec::new(),
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
