use api::{
    YTBus,
    protocol::{ApiCmd, ApiResponse},
};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

pub fn spawn_api_worker(
    mut api_cmd_rx: UnboundedReceiver<ApiCmd>,
    api_res_tx: UnboundedSender<ApiResponse>,
    bus: YTBus,
) {
    tokio::spawn(async move {
        while let Some(cmd) = api_cmd_rx.recv().await {
            let res = match cmd {
                ApiCmd::CreatePlaylist {
                    title,
                    description,
                    privacy,
                } => ApiResponse::CreatePlaylist(
                    bus.create_playlist(&title, &description, privacy).await,
                ),
                ApiCmd::SaveSong { song, playlist_id } => {
                    ApiResponse::SaveSong(match bus.save_to_playlist(&song, &playlist_id).await {
                        Ok(_) => Ok((song, playlist_id)),
                        Err(e) => Err(e),
                    })
                }
                ApiCmd::Search(query) => {
                    let (albums, songs) =
                        tokio::join!(bus.get_search_albums(&query), bus.get_search_songs(&query));
                    ApiResponse::Search { albums, songs }
                }
                ApiCmd::LikeSong(song) => ApiResponse::LikeSong(match bus.like_song(&song).await {
                    Ok(_) => Ok(song),
                    Err(e) => Err(e),
                }),
                ApiCmd::UnlikeSong(song) => {
                    let res = match bus.unlike_song(&song).await {
                        Ok(_) => Ok(()),
                        Err(e) => Err(e),
                    };
                    ApiResponse::UnlikeSong((res, song.title))
                }
                ApiCmd::UnsaveSong { song, playlist_id } => {
                    let res = match bus.unsave_to_playlist(&song, &playlist_id).await {
                        Ok(_) => Ok(()),
                        Err(e) => Err(e),
                    };
                    ApiResponse::UnsaveSong((res, song.title))
                }
                ApiCmd::GetSongsToView(playlist) => {
                    let songs = bus.get_songs(&playlist.browse_id).await;
                    ApiResponse::GetSongsToView { songs, playlist }
                }
                ApiCmd::GetSongsToPlay(playlist) => {
                    let songs = bus.get_songs(&playlist.browse_id).await;
                    ApiResponse::GetSongsToPlay {
                        songs,
                        playlist_id: playlist.playlist_id,
                    }
                }
                ApiCmd::UnsaveAlbum(playlist) => {
                    let res = match bus.unsave_album(&playlist.playlist_id).await {
                        Ok(_) => Ok(()),
                        Err(e) => Err(e),
                    };
                    ApiResponse::UnsaveAlbum((res, playlist))
                }
                ApiCmd::UnsaveCusPlaylist(playlist) => {
                    let res = match bus.unsave_cus_playlist(&playlist.playlist_id).await {
                        Ok(_) => Ok(()),
                        Err(e) => Err(e),
                    };
                    ApiResponse::UnsaveCusPlaylist((res, playlist.title))
                }
                ApiCmd::SaveAlbum(album) => {
                    let res = match bus.save_album(&album.playlist_id).await {
                        Ok(_) => Ok(()),
                        Err(e) => Err(e),
                    };
                    ApiResponse::SaveAlbum((res, album))
                }
                ApiCmd::GetRelatedSongsToPlay(song) => match bus.get_params(&song.video_id).await {
                    Ok(params) => {
                        let related_songs = bus.get_related_songs(song, &params).await;
                        ApiResponse::GetRelatedSongsToPlay(related_songs)
                    }
                    Err(e) => ApiResponse::GetRelatedSongsToPlay(Err(e)),
                },
                ApiCmd::FetchLibraryData => ApiResponse::FetchLibraryData(bus.get_lists().await),
            };
            if api_res_tx.send(res).is_err() {
                break;
            }
        }
    });
}
