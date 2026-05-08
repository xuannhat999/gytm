use data::Song;
use error::log_to_file;
use error::{Result, YError};
use serde::Deserialize;
use serde_json::Value;
use std::{
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
    event: Option<String>,
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
    pub async fn next(&mut self) -> Result<()> {
        self.state = PlayerState::Loading;
        self.send_mpv_command(r#"{"command": ["playlist-next"]}"#)
            .await?;
        Ok(())
    }

    // PLAY NEXT SONG IN ALBUM/PLAYLIST
    pub async fn prev(&mut self) -> Result<()> {
        self.state = PlayerState::Loading;
        self.send_mpv_command(r#"{"command": ["playlist-prev"]}"#)
            .await?;
        Ok(())
    }

    // PAUSE PLAYING SONG
    pub async fn toggle_pause(&mut self) -> Result<()> {
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
            .await?;
        Ok(())
    }

    pub fn start_mpv(&mut self) -> Result<()> {
        let child = Command::new("mpv")
            .arg("--idle")
            .arg(format!("--input-ipc-server={}", self.socket_path))
            .arg("--no-video")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|_| YError::MpvSpawnError)?;
        self.current_process = Some(child);
        Ok(())
    }

    async fn send_mpv_command(&self, command: &str) -> Result<()> {
        let mut stream = UnixStream::connect(&self.socket_path)
            .await
            .map_err(|e| YError::MpvSocketError(e.to_string()))?;
        stream.write_all(command.as_bytes()).await?;
        stream.write_all(b"\n").await?;
        Ok(())
    }
    pub async fn listen_playlist_changes(
        &self,
        tx: std::sync::mpsc::Sender<MpvEvent>,
    ) -> Result<()> {
        let socket_path = self.socket_path.clone();
        tokio::spawn(async move {
            let process_events = async {
                let mut s = UnixStream::connect(&socket_path)
                    .await
                    .map_err(|e| YError::MpvSocketError(e.to_string()))?;

                let observe_cmd = r#"{"command": ["observe_property", 1, "playlist"]}"#;
                s.write_all(format!("{}\n", observe_cmd).as_bytes())
                    .await
                    .map_err(YError::IoError)?;

                let reader = BufReader::new(s);
                let mut lines = reader.lines();

                while let Ok(Some(line)) = lines.next_line().await {
                    let msg: MpvResponse = serde_json::from_str(&line)?;
                    if msg.event.as_deref() == Some("property-change")
                        && msg.name.as_deref() == Some("playlist")
                    {
                        if let Some(items) = msg.data.and_then(|d| d.as_array().cloned()) {
                            let mpv_ids: Vec<String> = items
                                .iter()
                                .filter_map(|i| i["filename"].as_str().map(get_vid_id_from_url))
                                .collect();
                            tx.send(MpvEvent::ListChange(mpv_ids))
                                .map_err(|e| YError::ChannelSendError(e.to_string()))?;
                            if let Some(item) = items.iter().find(|i| {
                                i["playing"].as_bool() == Some(true)
                                    || i["current"].as_bool() == Some(true)
                            }) {
                                if let (Some(url), Some(title)) =
                                    (item["filename"].as_str(), item["title"].as_str())
                                {
                                    let song = Song {
                                        title: title.to_string(),
                                        video_id: get_vid_id_from_url(url),
                                        ..Default::default()
                                    };
                                    tx.send(MpvEvent::StartPlaying(song))
                                        .map_err(|e| YError::ChannelSendError(e.to_string()));
                                }
                            }
                        }
                    }
                }
                Ok::<(), YError>(())
            };
            if let Err(e) = process_events.await {
                log_to_file(&format!("Playlist Listener Error: {}\n", e));
            }
        });
        Ok(())
    }

    pub async fn toggle_playmode(&mut self) -> Result<()> {
        self.state = PlayerState::Loading;
        match self.play_mode {
            PlayMode::DefaultMode => {
                self.play_mode = PlayMode::ShuffleMode;
                self.send_mpv_command(r#"{"command": ["playlist-shuffle"]}"#)
                    .await?;
            }
            PlayMode::ShuffleMode => {
                self.play_mode = PlayMode::DefaultMode;
                self.send_mpv_command(r#"{"command": ["playlist-unshuffle"]}"#)
                    .await?;
            }
        }
        Ok(())
    }

    pub async fn shuffle(&self) -> Result<()> {
        self.send_mpv_command(r#"{"command": ["playlist-shuffle"]}"#)
            .await?;
        Ok(())
    }

    pub async fn load_playlist(&mut self, songs: &[Song]) -> Result<()> {
        if songs.is_empty() {
            return Err(YError::PlaylistEmpty);
        }
        self.state = PlayerState::Loading;
        if self.playlist_file.is_none() {
            self.playlist_file = Some(NamedTempFile::new()?);
        }
        if let Some(ref mut tempfile) = self.playlist_file {
            let file = tempfile.as_file_mut();
            file.set_len(0)?;
            std::io::Seek::seek(file, std::io::SeekFrom::Start(0))?;
            for song in songs {
                writeln!(file, "https://www.youtube.com/watch?v={}", song.video_id)?;
            }
            file.sync_all()?;
            let playlist_path = tempfile.path().to_string_lossy().to_string();
            let load_cmd = format!(
                r#"{{"command": ["loadlist", "{}", "replace"]}}{}"#,
                playlist_path, "\n"
            );
            self.send_mpv_command(&load_cmd).await?;
        }
        Ok(())
    }

    pub async fn play_at_idx(&mut self, index: &usize) -> Result<()> {
        self.state = PlayerState::Loading;
        let command = format!(
            r#"{{"command": ["set_property", "playlist-pos", {}]}}"#,
            index
        );
        self.send_mpv_command(&command).await?;
        Ok(())
    }
}

fn get_vid_id_from_url(url: &str) -> String {
    url.split("v=").last().unwrap_or(url).to_string()
}
