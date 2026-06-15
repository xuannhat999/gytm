use data::{MpvCommand, MpvEvent, MpvResponse, Song};
use error::{YError, YResult, log_to_file};
use std::{
    fs,
    io::Write,
    process::{Child, Command, Stdio},
};
use tempfile::NamedTempFile;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::UnixStream,
    sync::mpsc,
};

#[derive(Default)]
pub struct Player {
    current_process: Option<Child>,
    playlist_file: Option<NamedTempFile>,
    mpv_conn: Option<mpsc::UnboundedSender<MpvCommand>>,
}

impl Drop for Player {
    fn drop(&mut self) {
        self.kill_current_process();
    }
}

impl Player {
    // KILL CURRENT MPV PROCESS
    fn kill_current_process(&mut self) {
        if let Some(mut child) = self.current_process.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        self.playlist_file = None;
        self.mpv_conn = None;
        let _ = fs::remove_file(String::from("/tmp/gytm-mpv-socket"));
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
        let (tx, mut rx) = mpsc::unbounded_channel::<MpvCommand>();
        self.mpv_conn = Some(tx);

        tokio::spawn(async move {
            // CONNECT TO MPV SOCKET
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
                // PLAYLIST CHANGE
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
                // VOLUME CHANGE
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

                let reader = BufReader::new(raw_reader);
                let mut lines = reader.lines();

                let mut tick = tokio::time::interval(std::time::Duration::from_secs(1));
                tick.tick().await;
                loop {
                    tokio::select! {
                        // SEND TIME_POS CMD EVERY 1s
                        _ = tick.tick() => {
                            let cmd = r#"{"command": ["get_property", "time-pos"]}"#.to_string() + "\n";
                            if let Err(e) = writer.write_all(cmd.as_bytes()).await {
                                log_to_file(format!("Failed to send time-pos: {e}"));
                                break;
                            }
                        }
                        // RECEIVE CMD AND SEND TO SOCKET
                        cmd = rx.recv() => {
                            match cmd {
                                Some(mpv_cmd) => {
                                    let cmd = match_mpv_command(mpv_cmd);
                                    if !cmd.is_empty() {
                                        if let Err(e) = writer.write_all(cmd.as_bytes()).await {
                                            log_to_file(format!("Failed to send command to mpv: {e}"));
                                            break;
                                        }
                                    }
                                }
                                None => { break; }
                            }
                        },
                        // RECEIVE MPV RESPONSE AND SEND MPV EVENT
                        line = lines.next_line() => {
                            match line {
                                Ok(Some(ref l)) => {
                                    if let Ok(reply) = serde_json::from_str::<serde_json::Value>(l) {
                                        if reply.get("event").is_none() {
                                            if let (Some("success"), Some(v)) = (
                                                reply.get("error").and_then(|e| e.as_str()),
                                                reply.get("data").and_then(|d| d.as_f64()),
                                            ) {
                                                let _ = tx_event.send(MpvEvent::TimePos(v)).await;
                                            }
                                        }
                                    }
                                    if let Ok(msg) = serde_json::from_str::<MpvResponse>(l) {
                                        if msg.event.as_deref() == Some("property-change") {
                                            match msg.name.as_deref() {
                                                Some("playlist") => {
                                                    if let Some(items) =
                                                        msg.data.and_then(|d| d.as_array().cloned())
                                                    {
                                                        let mpv_ids: Vec<String> = items
                                                            .iter()
                                                            .filter_map(|i| {
                                                                i["filename"].as_str().map(|s|s.to_string())
                                                            })
                                                            .collect();
                                                        if let Err(e) =
                                                            tx_event.send(MpvEvent::ListChange(mpv_ids)).await
                                                        {
                                                            log_to_file(format!("Failed to send ListChange: {e}"));
                                                        }
                                                        if let Some(item) = items
                                                            .iter()
                                                            .find(|i| i["playing"].as_bool() == Some(true))
                                                        {
                                                            if let Some(url) = item["filename"].as_str() {
                                                                if let Err(e) = tx_event
                                                                    .send(MpvEvent::StartPlaying(url.to_string()))
                                                                    .await
                                                                {
                                                                    log_to_file(format!("Failed to send StartPlaying: {e}"));
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
                                                            log_to_file(format!("Failed to send VolumeChange: {e}"));
                                                        }
                                                    }
                                                }
                                                _ => {}
                                            }
                                        }
                                    }
                                }
                                Ok(None) => break,
                                Err(e) => {
                                    log_to_file(format!("MPV socket read error: {e}"));
                                    break;
                                }
                            }
                        }
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

    pub fn send_mpv_command(&self, command: MpvCommand) -> YResult<()> {
        if let Some(ref tx) = self.mpv_conn {
            tx.send(command)?;
        }
        Ok(())
    }

    pub fn write_tmp_list(&mut self, songs: &[Song]) -> YResult<String> {
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
            let file_path = tempfile.path().to_string_lossy().to_string();
            Ok(file_path)
        } else {
            Err(YError::InvalidFilePath)
        }
    }
}
fn match_mpv_command(mpv_cmd: MpvCommand) -> String {
    let cmd_str = match mpv_cmd {
        MpvCommand::Shuffle => r#"{"command": ["playlist-shuffle"]}"#,
        MpvCommand::Unshuffle => r#"{"command": ["playlist-unshuffle"]}"#,
        MpvCommand::PlayPrev => r#"{"command": ["playlist-prev"]}"#,
        MpvCommand::PlayNext => r#"{"command": ["playlist-next"]}"#,
        MpvCommand::SeekBackward => r#"{"command": ["seek", -5, "relative"]}"#,
        MpvCommand::SeekForward => r#"{"command": ["seek", 5, "relative"]}"#,
        MpvCommand::DecreaseVol => r#"{"command": ["add", "volume", -5]}"#,
        MpvCommand::IncreaseVol => r#"{"command": ["add", "volume", 5]}"#,
        MpvCommand::TogglePause => r#"{"command": ["cycle", "pause"]}"#,
        MpvCommand::SetVol(volume) => {
            return format!(r#"{{"command": ["set_property", "volume", {}]}}"#, volume) + "\n";
        }
        MpvCommand::PlayPos(pos) => {
            return format!(
                r#"{{"command": ["set_property", "playlist-pos", {}]}}"#,
                pos
            ) + "\n";
        }
        MpvCommand::LoadList(path) => {
            return format!(r#"{{"command": ["loadlist", "{}", "replace"]}}"#, path) + "\n";
        }
        MpvCommand::AppendSong(url) => {
            return format!(r#"{{"command": ["loadfile", "{}", "append-play"]}}"#, url) + "\n";
        }
        MpvCommand::RemovePos(idx) => {
            return format!(r#"{{"command": ["playlist-remove", {}]}}"#, idx) + "\n";
        }
        MpvCommand::Stop => r#"{"command": ["stop"]}"#,
        MpvCommand::Clear => r#"{"command": ["playlist-clear"]}"#,
    };
    if cmd_str.is_empty() {
        String::new()
    } else {
        format!("{}\n", cmd_str)
    }
}
