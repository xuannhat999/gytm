use error::{YError, YResult};
use std::{fs, path::PathBuf};

pub fn get_vid_id_from_url(url: &str) -> String {
    url.split("v=").last().unwrap_or(url).to_string()
}

pub fn get_url_from_vid_id(video_id: &str) -> String {
    format!("https://www.youtube.com/watch?v={}", video_id)
}

pub fn list_vid_id_from_list_url(urls: Vec<String>) -> Vec<String> {
    urls.iter().map(|u| get_vid_id_from_url(u)).collect()
}

pub fn format_time(secs: f64) -> String {
    let total = secs as u64;
    let h = total / 3600;
    let m = (total % 3600) / 60;
    let s = total % 60;
    if h > 0 {
        format!("{}:{:02}:{:02}", h, m, s)
    } else {
        format!("{}:{:02}", m, s)
    }
}

pub fn get_queue_file() -> YResult<PathBuf> {
    Ok(dirs::state_dir()
        .ok_or(YError::InvalidPath("~/.local/state/".to_string()))?
        .join("gytm/queue.json"))
}

pub fn remove_queue_file() {
    if let Ok(queue_file) = get_queue_file()
        && queue_file.exists()
    {
        fs::remove_file(queue_file).ok();
    }
}
