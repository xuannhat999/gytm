use crate::{app::App, helper};
use data::{app::PlayerStatus, mpv::MpvEvent};
use error::log_to_file;
use state::Persist;
use std::fs;

pub fn handle_mpv_event(app: &mut App, event: MpvEvent) {
    match event {
        MpvEvent::ListChange(list) => {
            let ids = helper::list_vid_id_from_list_url(&list);
            app.mpv_list = ids;
            app.save_queue_file().ok();
            fs::remove_file(data::file_path::MPV_PLAYLIST).ok();
        }
        MpvEvent::StartPlaying(url) => {
            let video_id = helper::get_vid_id_from_url(&url);
            let idx = app.queue.iter().position(|song| song.video_id == video_id);
            if idx != app.playing_song_idx {
                app.player_status = PlayerStatus::Playing;
                app.time_pos = Some(0.0);
            }
            app.playing_song_idx = idx;
        }
        MpvEvent::VolumeChange(vol) => {
            app.player_state.volume = vol;
            if let Err(e) = app.player_state.save() {
                log_to_file(&e);
            }
        }
        MpvEvent::TimePos(pos) => {
            app.time_pos = Some(pos);
        }
        MpvEvent::PauseChange(is_pause) => {
            if app.playing_song_idx.is_some() {
                if is_pause {
                    app.player_status = PlayerStatus::Paused
                } else {
                    app.player_status = PlayerStatus::Playing
                }
            }
        }
    }
}
