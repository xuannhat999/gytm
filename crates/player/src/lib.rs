use data::{MpvEvent, MpvResponse, PlayMode, PlayerStatus, Song};
use error::{YError, YResult, log_to_file};
use state::PlayerState;
use std::{
    io::Write,
    process::{Child, Command, Stdio},
};
use tempfile::NamedTempFile;
use tokio::net::UnixStream;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    sync::mpsc,
};

pub struct Player {
    pub current_process: Option<Child>,
    pub status: PlayerStatus,
    pub volume: u8,
    pub play_mode: PlayMode,
    pub playlist_file: Option<NamedTempFile>,
    mpv_conn: Option<mpsc::UnboundedSender<String>>,
}

impl Player {
    pub fn new(player_state: &PlayerState) -> Self {
        Self {
            current_process: None,
            status: PlayerStatus::Idle,
            volume: player_state.volume,
            play_mode: player_state.play_mode.clone(),
            playlist_file: None,
            mpv_conn: None,
        }
    }

    // KILL CURRENT MPV PROCESS
    pub fn kill_current_process(&mut self) {
        if let Some(mut child) = self.current_process.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        self.playlist_file = None;
        self.mpv_conn = None;
    }

    // PLAY PREVIOUS SONG IN ALBUM/PLAYLIST
    pub async fn next(&mut self) -> YResult<()> {
        self.status = PlayerStatus::Loading;
        self.send_mpv_command(r#"{"command": ["playlist-next"]}"#)
            .await?;
        Ok(())
    }

    // PLAY NEXT SONG IN ALBUM/PLAYLIST
    pub async fn prev(&mut self) -> YResult<()> {
        self.status = PlayerStatus::Loading;
        self.send_mpv_command(r#"{"command": ["playlist-prev"]}"#)
            .await?;
        Ok(())
    }

    // PAUSE PLAYING SONG
    pub async fn toggle_pause(&mut self) -> YResult<()> {
        match self.status {
            PlayerStatus::Playing => {
                self.status = PlayerStatus::Paused;
            }
            PlayerStatus::Paused => {
                self.status = PlayerStatus::Playing;
            }
            _ => {}
        };
        self.send_mpv_command(r#"{"command": ["cycle", "pause"]}"#)
            .await?;
        Ok(())
    }

    // START MPV SOCKET
    pub fn start_mpv_and_observe(
        &mut self,
        tx_event: tokio::sync::mpsc::Sender<MpvEvent>,
    ) -> YResult<()> {
        let socket_path = String::from("/tmp/gytm-mpv-socket");
        let child = Command::new("mpv")
            .arg("--idle")
            .arg(format!("--input-ipc-server={}", socket_path))
            .arg("--video=no")
            .arg(format!("--volume={}", self.volume))
            .arg("--loop-playlist=inf")
            .arg("--cache=yes")
            .arg("--cache-secs=5")
            .arg("--cache-on-disk=yes")
            .arg("--demuxer-max-bytes=5MiB")
            .arg("--demuxer-max-back-bytes=1MiB")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|_| YError::MpvSpawnError)?;

        self.current_process = Some(child);
        let (tx, mut rx) = mpsc::unbounded_channel::<String>();
        self.mpv_conn = Some(tx);

        // CONNECT TO MPV SOCKET
        tokio::spawn(async move {
            let mut stream: Option<UnixStream> = None;
            let mut retries = 0;
            const MAX_RETRIES: u32 = 50;

            while retries < MAX_RETRIES {
                match UnixStream::connect(&socket_path).await {
                    Ok(s) => {
                        stream = Some(s);
                        break;
                    }
                    Err(_) => {
                        retries += 1;
                        tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;
                    }
                }
            }
            if let Some(s) = stream {
                let (raw_reader, mut writer) = tokio::io::split(s);
                // SPAWN OBSERVE CMD
                if writer
                    .write_all(b"{\"command\": [\"observe_property\", 1, \"playlist\"]}\n")
                    .await
                    .is_err()
                {
                    log_to_file(YError::MpvSocketError(
                        "Failed to send observe playlist command".to_string(),
                    ));
                    return;
                }

                if writer
                    .write_all(b"{\"command\": [\"observe_property\", 2, \"volume\"]}\n")
                    .await
                    .is_err()
                {
                    log_to_file(YError::MpvSocketError(
                        "Failed to send observe volume command".to_string(),
                    ));
                    return;
                }
                // OBSERVE MPV RESPONSE AND SEND EVENT
                tokio::spawn(async move {
                    let reader = BufReader::new(raw_reader);
                    let mut lines = reader.lines();
                    while let Ok(Some(line)) = lines.next_line().await {
                        if let Ok(msg) = serde_json::from_str::<MpvResponse>(&line) {
                            if msg.event.as_deref() == Some("property-change") {
                                match msg.name.as_deref() {
                                    Some("playlist") => {
                                        if let Some(items) =
                                            msg.data.and_then(|d| d.as_array().cloned())
                                        {
                                            let mpv_ids: Vec<String> = items
                                                .iter()
                                                .filter_map(|i| {
                                                    i["filename"].as_str().map(get_vid_id_from_url)
                                                })
                                                .collect();
                                            if let Err(e) =
                                                tx_event.send(MpvEvent::ListChange(mpv_ids)).await
                                            {
                                                log_to_file(&e);
                                            }
                                            if let Some(item) = items.iter().find(|i| {
                                                i["playing"].as_bool() == Some(true)
                                                    || i["current"].as_bool() == Some(true)
                                            }) {
                                                if let (Some(url), Some(title)) = (
                                                    item["filename"].as_str(),
                                                    item["title"].as_str(),
                                                ) {
                                                    let song = Song {
                                                        title: title.to_string(),
                                                        video_id: get_vid_id_from_url(url),
                                                    };
                                                    if let Err(e) = tx_event
                                                        .send(MpvEvent::StartPlaying(song))
                                                        .await
                                                    {
                                                        log_to_file(&e);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    Some("volume") => {
                                        if let Some(volume) = msg.data.and_then(|d| d.as_f64()) {
                                            if let Err(e) = tx_event
                                                .send(MpvEvent::VolumeChange(volume as u8))
                                                .await
                                            {
                                                log_to_file(&e);
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                });

                // RECEIVE MPV COMMAND AND SEND TO SOCKET
                while let Some(cmd) = rx.recv().await {
                    if writer.write_all(cmd.as_bytes()).await.is_err() {
                        break;
                    }
                }
            } else {
                log_to_file(YError::MpvSocketError(
                    "Timeout: Cannot connect to MPV IPC socket".to_string(),
                ));
            }
        });
        Ok(())
    }

    pub async fn increase_volume(&mut self) -> YResult<()> {
        self.volume = self.volume.saturating_add(5).min(130);
        self.send_mpv_command(r#"{"command": ["add", "volume", 5]}"#)
            .await?;
        Ok(())
    }

    pub async fn decrease_volume(&mut self) -> YResult<()> {
        self.volume = self.volume.saturating_sub(5);
        self.send_mpv_command(r#"{"command": ["add", "volume", -5]}"#)
            .await?;
        Ok(())
    }
    async fn send_mpv_command(&self, command: &str) -> YResult<()> {
        if let Some(ref tx) = self.mpv_conn {
            let full_cmd = format!("{}\n", command);
            tx.send(full_cmd)?;
        }
        Ok(())
    }
    pub async fn toggle_playmode(&mut self) -> YResult<()> {
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

    pub async fn shuffle(&self) -> YResult<()> {
        self.send_mpv_command(r#"{"command": ["playlist-shuffle"]}"#)
            .await?;
        Ok(())
    }

    pub async fn load_playlist(&mut self, songs: &[Song]) -> YResult<()> {
        if songs.is_empty() {
            return Err(YError::PlaylistEmpty);
        }
        self.status = PlayerStatus::Loading;
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

    pub async fn remove_from_queue(&mut self, idx: usize) -> YResult<()> {
        let command = format!(r#"{{"command": ["playlist-remove", {}]}}"#, idx);
        self.send_mpv_command(&command).await?;
        Ok(())
    }

    pub async fn append_to_queue(&mut self, video_id: &str) -> YResult<()> {
        let command = format!(
            r#"{{"command": ["loadfile", "https://www.youtube.com/watch?v={}", "append"]}}"#,
            video_id
        );
        self.send_mpv_command(&command).await?;
        Ok(())
    }

    pub async fn play_at_idx(&mut self, index: &usize) -> YResult<()> {
        self.status = PlayerStatus::Loading;
        let command = format!(
            r#"{{"command": ["set_property", "playlist-pos", {}]}}"#,
            index
        );
        self.send_mpv_command(&command).await?;
        Ok(())
    }

    pub async fn clear_queue(&mut self) -> YResult<()> {
        self.status = PlayerStatus::Idle;
        self.send_mpv_command(r#"{"command": ["stop"]}"#).await?;
        self.send_mpv_command(r#"{"command": ["playlist-clear"]}"#)
            .await?;
        Ok(())
    }
}

fn get_vid_id_from_url(url: &str) -> String {
    url.split("v=").last().unwrap_or(url).to_string()
}
