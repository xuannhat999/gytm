use data::Song;
use std::process::{Command, Stdio};

#[derive(Default, PartialEq)]
pub enum PlayerState {
    #[default]
    Idle,
    Playing,
    Paused,
}
#[derive(Default)]
pub struct MusicPlayer {
    pub current_process: Option<std::process::Child>,
    pub state: PlayerState,
    pub volume: u8,
    pub current_song_idx: Option<usize>,
}

impl MusicPlayer {
    pub fn play_song(&mut self, video_id: &str) {
        self.kill_current_process();
        let child = Command::new("mpv")
            .arg(format!("https://www.youtube.com/watch?v={}", video_id))
            .arg("--no-video")
            .arg("--no-cache")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("Failed to start playback");

        self.current_process = Some(child);
        if self.state != PlayerState::Playing {
            self.state = PlayerState::Playing;
        }
    }

    pub fn kill_current_process(&mut self) {
        if let Some(mut child) = self.current_process.take() {
            let _ = child.kill();
        }
    }
    pub fn start_playlist(&mut self, songs: &[Song], start_index: usize) {
        self.kill_current_process();
        let mut command = Command::new("mpv");
        command
            .arg("--no-video")
            .arg("--cache=yes")
            .arg("--input-ipc-server=/tmp/mpv-socket")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .arg(format!("--playlist-start={}", start_index));

        for song in songs {
            command.arg(format!("https://www.youtube.com/watch?v={}", song.video_id));
        }

        let child = command.spawn().expect("Failed to start mpv");
        self.current_process = Some(child);
        self.state = PlayerState::Playing;
    }

    // Gửi lệnh qua Socket thay vì kill process
    fn send_ipc_command(&self, command: &str) {
        // Sử dụng lệnh socat hoặc ghi trực tiếp vào UnixStream trong Rust
        // Ví dụ nhanh bằng Command:
        let json_cmd = format!("{{ \"command\": [{}] }}\n", command);
        let _ = Command::new("sh")
            .arg("-c")
            .arg(format!("echo '{}' | socat - /tmp/mpv-socket", json_cmd))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
    }
    pub fn jump_to_index(&mut self, index: usize) {
        if self.state != PlayerState::Playing {
            self.state = PlayerState::Playing;
        }
        if self.current_song_idx != Some(index) {
            self.current_song_idx = Some(index);
        }
        let cmd = format!("\"set_property\", \"playlist-pos\", {}", index);
        self.send_ipc_command(&cmd);
    }
    pub fn next(&self) {
        self.send_ipc_command("\"playlist-next\"");
    }

    pub fn prev(&self) {
        self.send_ipc_command("\"playlist-prev\"");
    }
    pub fn resume(&mut self) {
        if self.state == PlayerState::Paused {
            // Gửi lệnh ép thuộc tính pause về false
            self.send_ipc_command("\"set_property\", \"pause\", false");
            self.state = PlayerState::Playing;
        }
    }

    // Hoặc sửa lại toggle_pause để cập nhật luôn state trong App
    pub fn toggle_pause(&mut self) {
        match self.state {
            PlayerState::Playing => {
                self.send_ipc_command("\"set_property\", \"pause\", true");
                self.state = PlayerState::Paused;
            }
            PlayerState::Paused => {
                self.send_ipc_command("\"set_property\", \"pause\", false");
                self.state = PlayerState::Playing;
            }
            _ => {}
        }
    }
}
