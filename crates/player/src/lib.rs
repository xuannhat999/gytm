use chrono::Local;
use data::Song;
use serde::Deserialize;
use serde_json::Value;
use std::{
    fs::OpenOptions,
    io::Write,
    process::{Command, Stdio},
};
use tempfile::NamedTempFile;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

pub enum MpvEvent {
    ListChange(Vec<String>),
    StartPlaying(Song),
}
#[derive(Deserialize, Debug)]
struct MpvResponse {
    event: String,
    name: Option<String>,
    #[serde(default)]
    data: Option<Value>,
}

#[derive(Default, PartialEq)]
pub enum PlayerState {
    #[default]
    Idle,
    Playing,
    Paused,
    Loading,
}

#[derive(Default, PartialEq)]
pub enum PlayMode {
    #[default]
    DefaultMode,
    ShuffleMode,
}

pub struct Player {
    pub current_process: Option<std::process::Child>,
    pub state: PlayerState,
    pub volume: u8,
    pub play_mode: PlayMode,
    pub socket_path: String,
    pub playlist_file: Option<NamedTempFile>,
}
impl Default for Player {
    fn default() -> Self {
        Self {
            current_process: None,
            state: PlayerState::Idle,
            volume: 100,
            play_mode: PlayMode::DefaultMode,
            socket_path: "/tmp/mpv-socket".to_string(),
            playlist_file: None,
        }
    }
}

impl Player {
    // KILL CURRENT MPV PROCESS
    pub fn kill_current_process(&mut self) {
        if let Some(mut child) = self.current_process.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        self.playlist_file = None
    }

    // PLAY PREVIOUS SONG IN ALBUM/PLAYLIST
    pub async fn next(&mut self) {
        self.state = PlayerState::Loading;
        self.send_mpv_command(r#"{"command": ["playlist-next"]}"#)
            .await;
    }

    // PLAY NEXT SONG IN ALBUM/PLAYLIST
    pub async fn prev(&mut self) {
        self.state = PlayerState::Loading;
        self.send_mpv_command(r#"{"command": ["playlist-prev"]}"#)
            .await;
    }

    // PAUSE PLAYING SONG
    pub async fn toggle_pause(&mut self) {
        match self.state {
            PlayerState::Playing => {
                self.state = PlayerState::Paused;
            }
            PlayerState::Paused => {
                self.state = PlayerState::Playing;
            }
            _ => {}
        };
        self.send_mpv_command(r#"{"command": ["cycle", "pause"]}"#)
            .await;
    }

    pub fn start_mpv(&mut self) {
        let child = Command::new("mpv")
            .arg("--idle")
            .arg(format!("--input-ipc-server={}", self.socket_path))
            .arg("--no-video")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("Failed to start mpv");
        self.current_process = Some(child);
    }

    async fn send_mpv_command(&self, command: &str) {
        if let Ok(mut stream) = UnixStream::connect(&self.socket_path).await {
            let _ = stream.write_all(command.as_bytes()).await;
            let _ = stream.write_all(b"\n").await;
        }
    }
    pub async fn listen_playlist_changes(&self, tx: std::sync::mpsc::Sender<MpvEvent>) {
        let socket_path = self.socket_path.clone();
        tokio::spawn(async move {
            let mut s = match UnixStream::connect(&socket_path).await {
                Ok(s) => s,
                Err(e) => return log_to_file(&format!("Error connect socket: {}", e)),
            };

            let observe_cmd = r#"{"command": ["observe_property", 1, "playlist"]}"#;
            if let Err(e) = s.write_all(format!("{}\n", observe_cmd).as_bytes()).await {
                return log_to_file(&format!("Error sending observe cmd: {}", e));
            }

            let reader = BufReader::new(s);
            let mut lines = reader.lines(); // Đây là Async lines

            while let Ok(Some(line)) = lines.next_line().await {
                let Ok(msg) = serde_json::from_str::<MpvResponse>(&line) else {
                    continue;
                };

                if msg.event == "property-change" && msg.name.as_deref() == Some("playlist") {
                    if let Some(items) = msg.data.and_then(|d| d.as_array().cloned()) {
                        let mpv_ids: Vec<String> = items
                            .iter()
                            .filter_map(|i| i["filename"].as_str().map(extract_id))
                            .collect();
                        let _ = tx.send(MpvEvent::ListChange(mpv_ids));
                        if let Some(item) = items.iter().find(|i| {
                            i["playing"].as_bool() == Some(true)
                                || i["current"].as_bool() == Some(true)
                        }) {
                            if let (Some(url), Some(title)) =
                                (item["filename"].as_str(), item["title"].as_str())
                            {
                                let song = Song {
                                    title: title.to_string(),
                                    video_id: extract_id(url),
                                    ..Default::default()
                                };
                                let _ = tx.send(MpvEvent::StartPlaying(song));
                            }
                        }
                    }
                }
            }
        });
    }

    // pub async fn load_song(&self, video_id: &str, append: bool) {
    //     let url = format!("https://www.youtube.com/watch?v={}", video_id);
    //
    //     let mode = if append { "append-play" } else { "replace" };
    //
    //     let command = serde_json::json!({
    //         "command": ["loadfile", url, mode]
    //     });
    //
    //     if let Ok(mut stream) = UnixStream::connect(&self.socket_path).await {
    //         if let Ok(cmd_string) = serde_json::to_string(&command) {
    //             let _ = writeln!(stream, "{}", cmd_string);
    //         }
    //     } else {
    //         eprintln!("Can not connect to socket");
    //     }
    // }

    pub async fn toggle_playmode(&mut self) {
        self.state = PlayerState::Loading;
        match self.play_mode {
            PlayMode::DefaultMode => {
                self.play_mode = PlayMode::ShuffleMode;
                self.send_mpv_command(r#"{"command": ["playlist-shuffle"]}"#)
                    .await;
                log_to_file("Shuffle");
            }
            PlayMode::ShuffleMode => {
                self.play_mode = PlayMode::DefaultMode;
                self.send_mpv_command(r#"{"command": ["playlist-unshuffle"]}"#)
                    .await;
                log_to_file("Default");
            }
        }
    }

    pub async fn shuffle(&self) {
        self.send_mpv_command(r#"{"command": ["playlist-shuffle"]}"#)
            .await;
    }

    pub async fn load_playlist(&mut self, songs: &[Song]) {
        if songs.is_empty() {
            return;
        }
        self.state = PlayerState::Loading;
        if self.playlist_file.is_none() {
            match NamedTempFile::new() {
                Ok(f) => self.playlist_file = Some(f),
                Err(e) => log_to_file(&format!("Failed to create temp file: {}", e)),
            }
        }
        if let Some(ref mut tempfile) = self.playlist_file {
            let file = tempfile.as_file_mut();
            let _ = file.set_len(0);
            let _ = std::io::Seek::seek(file, std::io::SeekFrom::Start(0));
            for song in songs {
                let _ = writeln!(file, "https://www.youtube.com/watch?v={}", song.video_id);
            }
            let _ = file.sync_all();
            let playlist_path = tempfile.path().to_string_lossy().to_string();
            if let Ok(mut stream) = tokio::net::UnixStream::connect(&self.socket_path).await {
                let load_cmd = format!(
                    r#"{{"command": ["loadlist", "{}", "replace"]}}{}"#,
                    playlist_path, "\n"
                );
                let _ = stream.write_all(load_cmd.as_bytes()).await;
                let _ = stream.flush().await;
            }
        }
    }

    pub async fn play_at_idx(&mut self, index: &usize) {
        self.state = PlayerState::Loading;
        let command = format!(
            r#"{{"command": ["set_property", "playlist-pos", {}]}}"#,
            index
        );
        log_to_file(&command);
        self.send_mpv_command(&command).await;
    }

    // pub async fn clear_playlist(&mut self) {
    //     self.state = PlayerState::Loading;
    //     self.send_mpv_command(r#"{"command": ["playlist-clear"]}"#)
    //         .await;
    // }
}

fn extract_id(url: &str) -> String {
    url.split("v=").last().unwrap_or(url).to_string()
}

pub fn log_to_file(message: &str) {
    let datetime = Local::now().format("%Y-%m-%d %H:%M:%S");
    let log_message = format!("{} : {}", datetime, message);
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open("log.txt") {
        let _ = file.write_all(log_message.as_bytes());
    }
}
