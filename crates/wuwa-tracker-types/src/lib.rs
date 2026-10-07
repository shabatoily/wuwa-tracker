//! crate 경계를 넘어 직렬화되는 뽑기 도메인 모델과 API 계약을 정의합니다.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// 뽑기 기록 API 요청에 필요한 URL payload입니다.
pub struct Payload {
    pub player_id: String,
    pub server_id: String,
    pub language_code: String,
    pub record_id: String,
    pub card_pool_id: String,
    pub card_pool_type: i32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// API 요청 payload와 배너별 수집 기록을 함께 보존하는 데이터입니다.
pub struct FetchResult {
    pub payload: Payload,
    /// key는 [`GachaType::key`]이며 각 기록은 최신순입니다.
    pub records: BTreeMap<String, Vec<Record>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GachaResponse {
    pub code: i32,
    pub message: String,
    pub data: Vec<Record>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// 게임 locale에서 사용하는 자원 종류와 배너 이름입니다.
pub struct LocaleData {
    #[serde(default)]
    pub character: String,
    #[serde(default)]
    pub weapon: String,
    #[serde(default)]
    pub item: String,
    /// key는 [`GachaType::key`], 값은 해당 locale의 배너 이름입니다.
    pub select_list: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// 뽑기 API가 반환하는 단일 획득 기록입니다.
pub struct Record {
    pub card_pool_type: String,
    pub resource_id: i32,
    pub quality_level: i32,
    pub resource_type: String,
    pub name: String,
    pub count: i32,
    pub time: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// 5성 획득과 해당 획득까지 누적된 pity입니다.
pub struct FiveStarRecord {
    pub name: String,
    pub time: String,
    pub pity: i32,
    pub is_pick_up: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// API 배너 식별자와 통계 계산 규칙입니다.
pub struct GachaType {
    pub id: i32,
    pub key: String,
    pub has_off_banner_drop: bool,
    pub name: String,
    /// 백분율 단위의 기본 5성 확률입니다.
    pub base_rate: f64,
    /// 운 점수 계산에 사용하는 픽업 5성 기대 뽑기 횟수입니다.
    pub expected_pulls: i32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// 배너 하나의 뽑기 기록과 파생 통계입니다.
pub struct Stats {
    pub gacha_type: i32,
    pub gacha_name: String,
    pub total_pulls: usize,
    pub total_astrite: usize,
    pub current_pity5: i32,
    pub current_pity4: i32,
    pub base_rate: f64,
    pub expected_pulls: i32,
    /// 최신순으로 정렬된 5성 획득 목록입니다.
    pub five_stars: Vec<FiveStarRecord>,
    /// 최신순으로 정렬된 원본 기록입니다.
    pub records: Vec<Record>,
    /// 완료된 픽업 획득 주기의 평균이며 픽뚫 비용을 포함합니다. 픽업이 없으면 0입니다.
    pub avg_pulls: f64,
    /// 실제 5성 획득률이며 백분율 단위입니다.
    pub actual_rate: f64,
    /// 완료된 픽업 주기의 기대 횟수를 실제 횟수로 나눈 점수입니다. 픽업이 없으면 0입니다.
    pub luck_score: f64,
    pub has_five_star: bool,
}

impl Stats {
    /// 평균과 운 점수를 평가할 수 있는 완료된 픽업 획득이 있는지 반환합니다.
    pub fn has_pick_up(&self) -> bool {
        self.five_stars.iter().any(|record| record.is_pick_up)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 여러 배너에 흩어진 5성 캐릭터 획득 기록의 요약입니다.
pub struct CharacterSummary {
    pub resource_id: i32,
    pub name: String,
    pub quality_level: i32,
    pub resource_type: String,
    pub copies: usize,
    /// 해당 캐릭터 획득까지 소비한 것으로 추정되는 별의 소리입니다.
    pub spent_astrite: usize,
    pub banner_count: usize,
    pub banners: Vec<String>,
    pub last_time: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// 운 점수에 대응하는 UI 상태의 시작 구간입니다.
pub struct LuckScoreThreshold {
    pub min_score: f64,
    pub state: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatsResponse {
    pub success: bool,
    #[serde(default)]
    pub player_id: String,
    #[serde(default)]
    pub stats: Vec<Stats>,
    #[serde(default)]
    pub character_summaries: Vec<CharacterSummary>,
    pub error: Option<String>,
    pub error_key: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanResponse {
    pub success: bool,
    #[serde(default)]
    pub url: String,
    pub error: Option<String>,
    pub error_key: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorResponse {
    pub success: bool,
    pub error: String,
    pub error_key: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigResponse {
    pub success: bool,
    #[serde(default)]
    pub luck_score_thresholds: Vec<LuckScoreThreshold>,
    #[serde(default)]
    pub resource_types: ResourceTypes,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceTypes {
    #[serde(default)]
    pub character: String,
    #[serde(default)]
    pub weapon: String,
    #[serde(default)]
    pub item: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayersResponse {
    pub success: bool,
    #[serde(default)]
    pub players: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportResponse {
    pub success: bool,
    pub filename: String,
    pub content_type: String,
    pub content: Vec<u8>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportData {
    pub player_id: String,
    pub stats: Vec<Stats>,
}
