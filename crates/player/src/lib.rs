use data::{MpvEvent, MpvResponse, PlayMode, PlayerStatus, Song};
use error::{Result, YError, log_to_file};
use state::PlayerState;
use std::{
    io::Write,
    process::{Command, Stdio},
};
use tempfile::NamedTempFile;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

pub struct Player {
    pub current_process: Option<std::process::Child>,
    pub state: PlayerStatus,
    pub volume: u8,
    pub play_mode: PlayMode,
    pub socket_path: String,
    pub playlist_file: Option<NamedTempFile>,
}

impl Player {
    pub fn new(player_state: &PlayerState) -> Self {
        Self {
            current_process: None,
            state: PlayerStatus::Idle,
            volume: player_state.volume,
            play_mode: player_state.play_mode.clone(),
            socket_path: "/tmp/mpv-socket".to_string(),
            playlist_file: None,
        }
    }

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
        self.state = PlayerStatus::Loading;
        self.send_mpv_command(r#"{"command": ["playlist-next"]}"#)
            .await?;
        Ok(())
    }

    // PLAY NEXT SONG IN ALBUM/PLAYLIST
    pub async fn prev(&mut self) -> Result<()> {
        self.state = PlayerStatus::Loading;
        self.send_mpv_command(r#"{"command": ["playlist-prev"]}"#)
            .await?;
        Ok(())
    }

    // PAUSE PLAYING SONG
    pub async fn toggle_pause(&mut self) -> Result<()> {
        match self.state {
            PlayerStatus::Playing => {
                self.state = PlayerStatus::Paused;
            }
            PlayerStatus::Paused => {
                self.state = PlayerStatus::Playing;
            }
            _ => {}
        };
        self.send_mpv_command(r#"{"command": ["cycle", "pause"]}"#)
            .await?;
        Ok(())
    }

    // START MPV SOCKET
    pub fn start_mpv(&mut self) -> Result<()> {
        let child = Command::new("mpv")
            .arg("--idle")
            .arg(format!("--input-ipc-server={}", self.socket_path))
            .arg("--no-video")
            .arg(format!("--volume={}", self.volume))
            .arg("--loop-playlist=inf")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|_| YError::MpvSpawnError)?;
        self.current_process = Some(child);
        Ok(())
    }

    pub async fn increase_volume(&mut self) -> Result<()> {
        self.volume = self.volume.saturating_add(5).min(100);
        self.send_mpv_command(r#"{"command": ["add", "volume", 5]}"#)
            .await?;
        Ok(())
    }

    pub async fn decrease_volume(&mut self) -> Result<()> {
        self.volume = self.volume.saturating_sub(5);
        self.send_mpv_command(r#"{"command": ["add", "volume", -5]}"#)
            .await?;
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
    pub async fn observe_mpv_changes(&self, tx: std::sync::mpsc::Sender<MpvEvent>) -> Result<()> {
        let socket_path = self.socket_path.clone();
        tokio::spawn(async move {
            let process_events = async {
                let mut s = UnixStream::connect(&socket_path)
                    .await
                    .map_err(|e| YError::MpvSocketError(e.to_string()))?;

                // OBSERVE PLAYLIST CHANGE (PLAYING SONG / PLAYING PLAYLIST)
                let observe_playlist_cmd = r#"{"command": ["observe_property", 1, "playlist"]}"#;
                s.write_all(format!("{}\n", observe_playlist_cmd).as_bytes())
                    .await
                    .map_err(YError::IoError)?;

                // OBSERVE VOLUME CHANGE
                let observe_vol_cmd = r#"{"command": ["observe_property", 2, "volume"]}"#;
                s.write_all(format!("{}\n", observe_vol_cmd).as_bytes())
                    .await
                    .map_err(YError::IoError)?;

                let reader = BufReader::new(s);
                let mut lines = reader.lines();

                while let Ok(Some(line)) = lines.next_line().await {
                    let msg: MpvResponse = serde_json::from_str(&line)?;
                    if msg.event.as_deref() == Some("property-change") {
                        match msg.name.as_deref() {
                            Some("playlist") => {
                                if let Some(items) = msg.data.and_then(|d| d.as_array().cloned()) {
                                    let mpv_ids: Vec<String> = items
                                        .iter()
                                        .filter_map(|i| {
                                            i["filename"].as_str().map(get_vid_id_from_url)
                                        })
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
                                            tx.send(MpvEvent::StartPlaying(song)).map_err(|e| {
                                                YError::ChannelSendError(e.to_string())
                                            })?;
                                        }
                                    }
                                }
                            }
                            Some("volume") => {
                                if let Some(volume) = msg.data.and_then(|d| d.as_f64()) {
                                    let vol = volume as u8;
                                    tx.send(MpvEvent::VolumeChange(vol))
                                        .map_err(|e| YError::ChannelSendError(e.to_string()))?;
                                }
                            }
                            _ => {}
                        }
                    }
                }
                Ok::<(), YError>(())
            };
            if let Err(e) = process_events.await {
                log_to_file(&e);
            }
        });
        Ok(())
    }

    pub async fn toggle_playmode(&mut self) -> Result<()> {
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
        self.state = PlayerStatus::Loading;
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
        self.state = PlayerStatus::Loading;
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
