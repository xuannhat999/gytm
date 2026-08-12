use data::app::PlayListPrivacy;
use serde::Serialize;

#[derive(Serialize)]
pub struct RequestContext<'a> {
    pub client: RequestClient<'a>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestClient<'a> {
    pub client_name: &'static str,
    pub client_version: &'a str,
}

#[derive(Serialize)]
pub struct CreatePlaylistRequest<'a> {
    pub context: RequestContext<'a>,
    pub title: &'a str,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<&'a str>,
    pub params: &'static str,

    #[serde(skip_serializing_if = "Option::is_none", rename = "privacyStatus")]
    pub privacy_status: Option<PlayListPrivacy>,
}

#[derive(Serialize)]
pub struct BrowseIdRequest<'a> {
    pub context: RequestContext<'a>,
    #[serde(rename = "browseId")]
    pub browse_id: &'a str,
}

#[derive(Serialize)]
pub struct GetContinuationRequest<'a> {
    pub context: RequestContext<'a>,
    pub continuation: &'a str,
}

#[derive(Serialize)]
pub struct QueryWithParamsRequest<'a> {
    pub context: RequestContext<'a>,
    pub query: &'a str,
    pub params: &'a str,
}
#[derive(Serialize)]
pub struct QueryRequest<'a> {
    pub context: RequestContext<'a>,
    pub query: &'a str,
}

#[derive(Serialize)]
pub struct VideoIdRequest<'a> {
    pub context: RequestContext<'a>,
    #[serde(rename = "videoId")]
    pub video_id: &'a str,
}

#[derive(Serialize)]
pub struct PlaylistIdRequest<'a> {
    pub context: RequestContext<'a>,
    #[serde(rename = "playlistId")]
    pub playlist_id: &'a str,
}

#[derive(Serialize)]
pub struct GetRelatedSongsRequest<'a> {
    pub context: RequestContext<'a>,
    #[serde(rename = "playlistId")]
    pub playlist_id: &'a str,
    pub params: &'a str,
    #[serde(rename = "tunerSettingValue")]
    pub tuner_setting_value: &'static str,
}

#[derive(Serialize)]
pub struct TargetRequest<'a, T: Serialize> {
    pub context: RequestContext<'a>,
    pub target: T,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetContent<'a> {
    #[serde(skip_serializing_if = "Option::is_none", rename = "playlistId")]
    pub playlist_id: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "videoId")]
    pub video_id: Option<&'a str>,
}
#[derive(Serialize)]
pub struct SaveAlbumRequest<'a, T: Serialize> {
    pub context: RequestContext<'a>,
    pub target: T,
    pub status: &'a str,
}

#[derive(Serialize)]
pub struct SaveUnsaveListRequest<'a> {
    pub context: RequestContext<'a>,
    pub actions: Vec<ActionsContent<'a>>,
    #[serde(rename = "playlistId")]
    pub playlist_id: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionsContent<'a> {
    pub action: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub added_video_id: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub removed_video_id: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub set_video_id: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dedupe_option: Option<&'a str>,
}
