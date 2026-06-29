use data::{MpvCommand, MpvEvent, MpvResponse, Song};
use error::{YError, YResult, log_to_file};
use std::{
    fs,
    io::Write,
    os::unix::process::CommandExt,
    process::{Command, Stdio},
};

use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::UnixStream,
    sync::mpsc,
};

#[derive(Default)]
pub struct Player {
    mpv_cmd_sender: Option<mpsc::UnboundedSender<MpvCommand>>,
}
impl Player {
    pub fn spawn_mpv(&mut self) -> YResult<()> {
        let _ = std::fs::remove_file(data::file_path::MPV_SOCKET);
        Command::new("mpv")
            .arg("--idle")
            .arg(format!(
                "--input-ipc-server={}",
                data::file_path::MPV_SOCKET
            ))
            .arg("--video=no")
            .arg("--loop-playlist=inf")
            .arg("--cache=yes")
            .arg("--cache-secs=5")
            .arg("--cache-on-disk=yes")
            .arg("--demuxer-max-bytes=5MiB")
            .arg("--demuxer-max-back-bytes=1MiB")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .process_group(0)
            .spawn()
            .map_err(|_| YError::MpvSpawnError)?;
        Ok(())
    }
    pub fn shutdown(&mut self) {
        if let Err(e) = self.send_mpv_command(MpvCommand::Quit) {
            log_to_file(&e);
        }
        self.mpv_cmd_sender = None;
        let _ = fs::remove_file(data::file_path::MPV_SOCKET);
    }

    // OBSERVE MPV SOCKET
    pub async fn observe_mpv(
        &mut self,
        stream: UnixStream,
        tx_event: tokio::sync::mpsc::Sender<MpvEvent>,
    ) -> YResult<()> {
        let (tx, mut rx) = mpsc::unbounded_channel::<MpvCommand>();
        self.mpv_cmd_sender = Some(tx);
        tokio::spawn(async move {
            let (raw_reader, mut writer) = tokio::io::split(stream);
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
            if writer
                .write_all(b"{\"command\": [\"observe_property\", 3, \"pause\"]}\n")
                .await
                .is_err()
            {
                log_to_file(YError::MpvSocketError(
                    "Failed to send observe pause command".to_string(),
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
                                if let Ok(msg) = serde_json::from_str::<MpvResponse>(l) {
                                    match msg.event.as_deref() {
                                        None => {
                                            if let Some(v) = msg.data.and_then(|d| d.as_f64()) {
                                                let _ = tx_event.send(MpvEvent::TimePos(v)).await;
                                            }
                                        }
                                        Some("property-change") => match msg.name.as_deref() {
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
                                            Some("pause") => {
                                                if let Some(is_paused) = msg.data.and_then(|d| d.as_bool()) {
                                                    if let Err(e) = tx_event
                                                        .send(MpvEvent::PauseChange(is_paused))
                                                        .await
                                                    {
                                                        log_to_file(format!("Failed to send PauseChange: {e}"));
                                                    }
                                                }
                                            }
                                            _ => {}
                                        },
                                        _ => {}
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
        });
        Ok(())
    }

    pub async fn connect_mpv(&mut self) -> YResult<UnixStream> {
        let mut stream: Option<UnixStream> = None;
        let mut retries = 0;
        const MAX_RETRIES: u32 = 50;
        while retries < MAX_RETRIES {
            match UnixStream::connect(data::file_path::MPV_SOCKET).await {
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
        match stream {
            Some(s) => Ok(s),
            None => Err(YError::MpvSocketError("Connect to mpv failed".to_string())),
        }
    }
    pub fn check_socket_exists() -> YResult<()> {
        if !std::path::Path::new(data::file_path::MPV_SOCKET).exists() {
            return Err(YError::MpvSocketError("MPV socket not found".to_string()));
        }
        Ok(())
    }

    pub fn send_mpv_command(&self, command: MpvCommand) -> YResult<()> {
        if let Some(ref tx) = self.mpv_cmd_sender {
            tx.send(command)?;
        }
        Ok(())
    }

    pub fn write_playlist(&self, songs: &[Song]) -> YResult<()> {
        let path = data::file_path::MPV_PLAYLIST;
        let mut file = fs::File::create(path)?;
        for song in songs {
            writeln!(file, "https://www.youtube.com/watch?v={}", song.video_id)?;
        }
        Ok(())
    }
}
fn match_mpv_command(mpv_cmd: MpvCommand) -> String {
    let cmd_str = match mpv_cmd {
        MpvCommand::Shuffle => r#"{"command": ["playlist-shuffle"]}"#,
        MpvCommand::Unshuffle => r#"{"command": ["playlist-unshuffle"]}"#,
        MpvCommand::PlayNext => {
            return r#"{"command": ["playlist-next", "force"]}"#.to_string()
                + "\n"
                + r#"{"command": ["set_property", "pause", false]}"#
                + "\n";
        }

        MpvCommand::PlayPrev => {
            return r#"{"command": ["playlist-prev", "force"]}"#.to_string()
                + "\n"
                + r#"{"command": ["set_property", "pause", false]}"#
                + "\n";
        }
        MpvCommand::SeekBackward(secs) => {
            return format!(r#"{{"command": ["seek", -{}, "relative"]}}"#, secs) + "\n";
        }
        MpvCommand::SeekForward(secs) => {
            return format!(r#"{{"command": ["seek", {}, "relative"]}}"#, secs) + "\n";
        }
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
        MpvCommand::LoadList => {
            return format!(
                r#"{{"command": ["loadlist", "{}", "replace"]}}"#,
                data::file_path::MPV_PLAYLIST
            ) + "\n";
        }
        MpvCommand::AppendSong(url) => {
            return format!(r#"{{"command": ["loadfile", "{}", "append-play"]}}"#, url) + "\n";
        }
        MpvCommand::RemovePos(idx) => {
            return format!(r#"{{"command": ["playlist-remove", {}]}}"#, idx) + "\n";
        }
        MpvCommand::Stop => r#"{"command": ["stop"]}"#,
        MpvCommand::Clear => r#"{"command": ["playlist-clear"]}"#,
        MpvCommand::Quit => r#"{"command": ["quit"]}"#,
    };
    if cmd_str.is_empty() {
        String::new()
    } else {
        format!("{}\n", cmd_str)
    }
}
