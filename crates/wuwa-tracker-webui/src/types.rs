use serde::Serialize;

pub use wuwa_tracker_types::{
    CharacterSummary, ConfigResponse, ExportResponse, FiveStarRecord, LuckScoreThreshold,
    PlayersResponse, Record, ScanResponse, Stats, StatsResponse,
};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerArgs<'a> {
    pub player_id: &'a str,
}

#[derive(Serialize)]
pub struct PathArgs<'a> {
    pub path: &'a str,
}

#[derive(Serialize)]
pub struct UrlArgs<'a> {
    pub url: &'a str,
}

#[derive(Serialize)]
pub struct UploadArgs<'a> {
    pub data: &'a serde_json::Value,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportArgs<'a> {
    pub player_id: &'a str,
    pub format: &'a str,
    pub lang: &'a str,
}
