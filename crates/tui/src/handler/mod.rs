mod api_response_event;
mod key_event;
mod mpv_event;

pub use api_response_event::handle_api_response;
pub use key_event::handle_key_events;
pub use mpv_event::handle_mpv_event;

use crate::{app::App, helper::get_url_from_vid_id, notification::NotifyType};
use data::{
    app::{PlayMode, PlayerStatus, Song},
    mpv::MpvCommand,
};
use error::{YResult, log_to_file};
use player::Player;

fn append_song_to_queue(app: &mut App, player: &Player, song: Song) -> YResult<()> {
    if app.queue.iter().any(|s| s.video_id == song.video_id) {
        app.noti.notify(
            NotifyType::Error,
            format!("'{}' already in queue", song.title),
        );
        return Ok(());
    }
    let url = get_url_from_vid_id(&song.video_id);
    player.send_mpv_command(MpvCommand::AppendSong(url))?;
    if app.player_state.play_mode == PlayMode::ShuffleMode && app.queue.len() == 3 {
        player.send_mpv_command(MpvCommand::Shuffle)?;
    }
    app.noti.notify(
        NotifyType::Success,
        format!("Appended '{}' in queue", song.title),
    );
    app.queue.push(song);

    Ok(())
}

pub(crate) fn remove_song_from_queue(
    app: &mut App,
    player: &mut Player,
    idx: usize,
    mpv_idx: usize,
) {
    if let Err(e) = player.send_mpv_command(MpvCommand::RemovePos(mpv_idx)) {
        log_to_file(&e);
    } else {
        if let Some(playing_idx) = app.playing_song_idx {
            if playing_idx > idx {
                app.playing_song_idx = Some(playing_idx - 1);
            } else if playing_idx == idx {
                app.playing_song_idx = None;
            }
        }
        app.queue.remove(idx);
        if app.queue.is_empty() {
            app.player_status = PlayerStatus::Idle;
            app.playing_playlist_id = None;
            app.time_pos = None;
        }
        app.noti
            .notify(NotifyType::Success, String::from("Removed song from Queue"));
    }
}

pub(crate) fn clear_queue(app: &mut App, player: &Player) -> YResult<()> {
    player.send_mpv_command(MpvCommand::Clear)?;
    app.player_status = PlayerStatus::Idle;
    app.playing_song_idx = None;
    app.time_pos = None;
    app.queue.clear();
    app.playing_playlist_id = None;

    Ok(())
}

pub(crate) fn load_list(
    app: &mut App,
    player: &Player,
    songs: Vec<Song>,
    start_index: usize,
    playlist_id: Option<String>,
) -> YResult<()> {
    if !songs.is_empty() {
        player.write_playlist(&songs)?;
        player.send_mpv_command(MpvCommand::LoadList)?;
        if start_index > 0 {
            player.send_mpv_command(MpvCommand::PlayPos(start_index))?;
        }
        if app.player_state.play_mode == PlayMode::ShuffleMode {
            player.send_mpv_command(MpvCommand::Shuffle)?;
        }
        app.queue = songs;
        app.queue_liststate.select(Some(start_index));
        app.playing_playlist_id = playlist_id;
        app.playing_song_idx = None;
    } else {
        clear_queue(app, player).ok();
    }
    Ok(())
}
