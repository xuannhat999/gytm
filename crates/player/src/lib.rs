use serde::Deserialize;
use serde_json::Value;
use std::{
    io::{BufRead, BufReader, Write},
    os::unix::net::UnixStream,
    process::{Command, Stdio},
    thread,
};

pub enum MpvEvent {
    ListChange(Value),
    EndSong,
    StartPlaying(String),
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
}
impl Default for Player {
    fn default() -> Self {
        Self {
            current_process: None,
            state: PlayerState::Idle,
            volume: 100,
            play_mode: PlayMode::DefaultMode,
            socket_path: "/tmp/mpv-socket".to_string(),
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
    }

    // SEND COMMAND TO IPC
    fn send_ipc_command(&self, command: &str) {
        let json_cmd = format!("{{ \"command\": [{}] }}\n", command);
        let _ = Command::new("sh")
            .arg("-c")
            .arg(format!(
                "echo '{}' | socat - {}",
                json_cmd, self.socket_path
            ))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
    }

    // PLAY A SONG IN CURRENT ALBUM/PLAYLIST
    pub fn jump_to_index(&mut self, index: usize) {
        if self.state != PlayerState::Playing {
            self.state = PlayerState::Playing;
        }
        let cmd = format!("\"set_property\", \"playlist-pos\", {}", index);
        self.send_ipc_command(&cmd);
    }

    // PLAY PREVIOUS SONG IN ALBUM/PLAYLIST
    pub fn next(&self) {
        self.send_mpv_command(r#"{"command": ["playlist-next"]}"#);
    }

    // PLAY NEXT SONG IN ALBUM/PLAYLIST
    pub fn prev(&self) {
        self.send_mpv_command(r#"{"command": ["playlist-prev"]}"#);
    }

    // PAUSE PLAYING SONG
    pub fn toggle_pause(&mut self) {
        match self.state {
            PlayerState::Playing => {
                self.state = PlayerState::Paused;
            }
            PlayerState::Paused => {
                self.state = PlayerState::Playing;
            }
            _ => {}
        };
        self.send_mpv_command(r#"{"command": ["cycle", "pause"]}"#);
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

    fn send_mpv_command(&self, command: &str) {
        if let Ok(mut stream) = UnixStream::connect(&self.socket_path) {
            let _ = stream.write_all(command.as_bytes());
            let _ = stream.write_all(b"\n");
        }
    }

    pub fn listen_playlist_changes(&self, tx: std::sync::mpsc::Sender<MpvEvent>) {
        let socket_path = self.socket_path.clone();
        thread::spawn(move || {
            let mut stream = UnixStream::connect(&socket_path).unwrap();

            // Chỉ cần quan sát playlist
            let observe_cmd = r#"{"command": ["observe_property", 1, "playlist"]}"#;
            let _ = writeln!(stream, "{}", observe_cmd);

            let reader = BufReader::new(stream);
            for line in reader.lines() {
                if let Ok(line_str) = line {
                    if let Ok(msg) = serde_json::from_str::<MpvResponse>(&line_str) {
                        if msg.event == "property-change" && msg.name.as_deref() == Some("playlist")
                        {
                            if let Some(data) = msg.data {
                                // if let Ok(pretty) = serde_json::to_string_pretty(&data) {
                                //     println!("{}", pretty);
                                // }
                                if let Some(items) = data.as_array() {
                                    if let Some(current_item) = items.iter().find(|i| {
                                        i["current"].as_bool() == Some(true)
                                            && i["playing"].as_bool() == Some(true)
                                    }) {
                                        // println!("Send StartPlayingEvent");
                                        if let Some(url) = current_item["filename"].as_str() {
                                            let video_id = extract_id(url);
                                            let _ = tx.send(MpvEvent::StartPlaying(video_id));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });
    }
    pub fn load_song(&self, video_id: &str, append: bool) {
        let url = format!("https://www.youtube.com/watch?v={}", video_id);

        let mode = if append { "append-play" } else { "replace" };

        let command = serde_json::json!({
            "command": ["loadfile", url, mode]
        });

        if let Ok(mut stream) = UnixStream::connect(&self.socket_path) {
            if let Ok(cmd_string) = serde_json::to_string(&command) {
                let _ = writeln!(stream, "{}", cmd_string);
            }
        } else {
            eprintln!("Can not connect to socket");
        }
    }
}
fn extract_id(url: &str) -> String {
    url.split("v=").last().unwrap_or(url).to_string()
}
