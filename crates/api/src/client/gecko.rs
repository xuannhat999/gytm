use std::{
    fs,
    path::{Path, PathBuf},
};

use data::api_client::{BrowserProfile, GeckoContainer};
use error::{YError, YResult};
use ini::Ini;
use reqwest::{Url, cookie::Jar};
use rusqlite::Connection;

use crate::{client::get_file_path_from_root_and_filename, dao::YTM_DOMAIN};

pub fn get_geckgo_containers_from_profile(profile_path: &Path) -> YResult<Vec<GeckoContainer>> {
    let file_path = get_file_path_from_root_and_filename(profile_path, "containers.json");
    if let Some(file) = file_path {
        return get_gecko_containers_from_json(&file);
    }
    Ok(Vec::new())
}

pub(crate) fn read_gecko_cookies(
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

                let prefix_pattern = format!("^userContextId={}&%", container_id);

                let rows = stmt.query_map(
                    rusqlite::params![host_pattern, exact_pattern, prefix_pattern],
                    |row| {
                        Ok(GeckoCookie {
                            name: row.get(0)?,
                            value: row.get(1)?,
                            host: row.get(2)?,
                            path: row.get(3)?,
                            expires: row.get(4)?,
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
                        expires: row.get(4)?,
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

pub(crate) fn filter_exp_gecko_cookies(cookies: Vec<GeckoCookie>) -> Vec<GeckoCookie> {
    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    cookies
        .into_iter()
        .filter(|c| {
            let not_expired = c.expires > now_secs.max(0) || c.expires == 0;
            not_expired && !c.name.is_empty() && !c.value.is_empty()
        })
        .collect()
}

pub(crate) fn build_jar_from_gecko_cookies(cookies: Vec<GeckoCookie>) -> YResult<(Jar, String)> {
    let url = YTM_DOMAIN.parse::<Url>()?;
    let jar = Jar::default();
    let mut sapisid: Option<String> = None;
    for cookie in cookies {
        if cookie.name == "SAPISID" {
            sapisid = Some(cookie.value.clone());
        }
        let cookie_str = format!(
            "{}={}; Path={}; Secure; HttpOnly",
            cookie.name, cookie.value, cookie.path
        );
        jar.add_cookie_str(&cookie_str, &url);
    }
    sapisid
        .map(|sapisid| (jar, sapisid))
        .ok_or(YError::InvalidCookie)
}

pub(crate) fn get_gecko_profiles_from_sqlite(root_path: &Path) -> YResult<Vec<BrowserProfile>> {
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

pub(crate) fn get_gecko_profiles_from_ini(root_path: &Path) -> YResult<Vec<BrowserProfile>> {
    let ini_path = get_file_path_from_root_and_filename(root_path, "profiles.ini");
    let mut profiles = Vec::new();
    if let Some(ini_path) = ini_path {
        let ini = Ini::load_from_file(&ini_path)?;
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
    }
    Ok(profiles)
}

fn get_gecko_containers_from_json(json_path: &Path) -> YResult<Vec<GeckoContainer>> {
    let content = fs::read_to_string(json_path)?;
    let json = gjson::get(&content, "identities");
    let mut containers = vec![GeckoContainer {
        id: None,
        name: "None".to_string(),
    }];
    json.each(|_, val| {
        let is_public = val.get("public").bool();
        if is_public {
            let name = val.get("name");
            let l10n_id = val.get("l10nId");
            let id = val.get("userContextId").i32();
            let container_name = if name.exists() {
                name.str().to_string()
            } else {
                l10n_id.str().to_string()
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeckoCookie {
    pub(crate) name: String,
    pub(crate) value: String,
    pub(crate) host: String,
    pub(crate) path: String,
    pub(crate) expires: i64,
    pub(crate) is_secure: bool,
    pub(crate) is_http_only: bool,
    pub(crate) origin_attributes: String,
}
