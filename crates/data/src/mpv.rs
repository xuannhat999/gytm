use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize, Debug)]
pub struct MpvResponse {
    pub event: Option<String>,
    pub name: Option<String>,
    pub data: Option<Value>,
}

pub enum MpvEvent {
    ListChange(Vec<String>),
    StartPlaying(String),
    VolumeChange(u8),
    TimePos(f64),
    PauseChange(bool),
}

#[derive(PartialEq)]
pub enum MpvCommand {
    Shuffle,
    Unshuffle,
    SeekForward(i64),
    SeekBackward(i64),
    PlayNext,
    PlayPrev,
    TogglePause,
    IncreaseVol,
    DecreaseVol,
    SetVol(u8),
    PlayPos(usize),
    AppendSong(String),
    LoadList,
    RemovePos(usize),
    Stop,
    Clear,
}
