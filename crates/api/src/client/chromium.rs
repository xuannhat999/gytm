use std::path::Path;

use data::api_client::BrowserProfile;
use error::{YError, YResult};
use reqwest::{Url, cookie::Jar};
use rookie::enums::Cookie;

use crate::{YTM_DOMAIN, YTM_URL, client};

use client::get_file_path_from_root_and_filename;

pub(crate) fn get_chromium_profiles_from_root(root: &Path) -> YResult<Vec<BrowserProfile>> {
    let local_state_path = get_file_path_from_root_and_filename(root, "Local State");
    if let Some(local_state) = local_state_path {
        let content = std::fs::read_to_string(&local_state)?;
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

pub(crate) fn read_chromium_cookies(db_path: &Path, key_path: &Path) -> YResult<Vec<Cookie>> {
    let domains = vec![YTM_DOMAIN.to_string()];
    let str_db_path = db_path.to_string_lossy().into_owned();
    let str_key_path = key_path.to_string_lossy().into_owned();
    let cookies = rookie::any_browser(&str_db_path, Some(domains), Some(&str_key_path));
    match cookies {
        Ok(c) => Ok(c),
        Err(e) => Err(YError::RookieError(e.to_string())),
    }
}

pub(crate) fn filter_exp_chromium_cookies(cookies: Vec<Cookie>) -> YResult<Vec<Cookie>> {
    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let valid_cookies: Vec<Cookie> = cookies
        .into_iter()
        .filter(|c| {
            let not_expired = match c.expires {
                None => true,
                Some(exp) => exp > (now_secs.max(0) as u64),
            };
            not_expired && !c.name.is_empty() && !c.value.is_empty()
        })
        .collect();
    if valid_cookies.is_empty() {
        return Err(YError::CookieExpired);
    }
    Ok(valid_cookies)
}

pub(crate) fn build_jar_sapisid_from_chromium_cookies(
    cookies: Vec<Cookie>,
) -> YResult<(Jar, String)> {
    let url = YTM_URL.parse::<Url>()?;
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
