use std::{
    collections::BTreeMap,
    fs,
    path::Path,
    sync::{Arc, RwLock},
};
use tracing::{debug, error, info, warn};
use wuwa_tracker_core::{
    characters::character_summaries,
    config::Config,
    error::AppError,
    reporter::{self, ReportFormat},
    scanner,
    stats::StatsCalculator,
    store::{RedbStore, StoreStats},
    tracker::{self, TrackerClient},
};
use wuwa_tracker_types::{
    CharacterSummary, FetchResult, GachaType, LocaleData, Payload, Record, ReportData,
    ResourceTypes, ScanResponse, StatsResponse,
};

#[derive(Clone)]
/// 모든 진입점이 공유하는 애플리케이션 작업과 운영 로그의 경계입니다.
///
/// clone된 인스턴스는 저장소와 현재 locale 상태를 공유합니다.
pub struct Service {
    config: Arc<Config>,
    store: Arc<RedbStore>,
    calc: StatsCalculator,
    tracker: TrackerClient,
    locale: Arc<RwLock<Option<LocaleData>>>,
}

#[derive(Debug, Clone)]
pub struct BannerRecordCount {
    pub id: i32,
    pub key: String,
    pub name: String,
    pub records: usize,
}

impl Service {
    /// 설정에 지정된 내장 저장소를 열고 외부 API client를 준비합니다.
    ///
    /// # Errors
    ///
    /// 저장소 파일을 열지 못하면 [`AppError`]를 반환합니다.
    pub fn new(config: Config) -> Result<Self, AppError> {
        let calc = StatsCalculator::new(&config);
        let store = Arc::new(RedbStore::new(config.db_path.clone())?);
        let tracker = TrackerClient::new(config.resources_url.clone(), config.tracking_url.clone());
        let service = Self {
            config: Arc::new(config),
            store,
            calc,
            tracker,
            locale: Arc::new(RwLock::new(None)),
        };
        info!(
            event = "service_initialized",
            db_path = %service.config.db_path.display(),
            log_path = %service.config.log_path.display(),
        );
        Ok(service)
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    /// 원격 locale이 준비되지 않았으면 설정 언어의 내장 locale을 사용합니다.
    pub fn resource_types(&self) -> ResourceTypes {
        let locale = self
            .locale
            .read()
            .expect("locale lock poisoned")
            .clone()
            .or_else(|| tracker::load_local_gacha_locale(&self.config.language).ok())
            .unwrap_or_default();
        ResourceTypes {
            character: locale.character,
            weapon: locale.weapon,
            item: locale.item,
        }
    }

    pub fn list_players(&self) -> Vec<String> {
        self.store.list_players()
    }

    pub fn store_stats(&self) -> Result<StoreStats, AppError> {
        let result = self.store.stats();
        match &result {
            Ok(stats) => info!(
                event = "store_stats_loaded",
                players = stats.players,
                banners = stats.banners,
                records = stats.records,
                size_bytes = stats.size_bytes,
            ),
            Err(error) => error!(event = "store_stats_failed", error = %error),
        }
        result
    }

    pub fn banner_record_counts(
        &self,
        player_id: impl AsRef<str>,
    ) -> Result<Vec<BannerRecordCount>, AppError> {
        let player_id = player_id.as_ref().trim();
        if player_id.is_empty() {
            return Err(AppError::MissingPlayerId);
        }
        if !self.store.has_player(player_id) {
            return Err(AppError::PlayerNotFound);
        }

        let result: Result<Vec<BannerRecordCount>, AppError> = self
            .config
            .gacha_types
            .iter()
            .map(|gacha_type| {
                let records = self.store.get_gacha_records(player_id, &gacha_type.key)?;
                let gacha_type = self.localized_gacha_type(gacha_type);
                Ok(BannerRecordCount {
                    id: gacha_type.id,
                    key: gacha_type.key,
                    name: gacha_type.name,
                    records: records.len(),
                })
            })
            .collect();
        match &result {
            Ok(counts) => info!(
                event = "banner_record_counts_loaded",
                player_id = %player_id,
                banners = counts.len(),
            ),
            Err(error) => error!(
                event = "banner_record_counts_failed",
                error = %error,
                player_id = %player_id,
            ),
        }
        result
    }

    pub fn character_summaries(
        &self,
        player_id: impl AsRef<str>,
    ) -> Result<Vec<CharacterSummary>, AppError> {
        let player_id = player_id.as_ref();
        let result = self
            .load_player_stats(player_id)
            .map(|response| response.character_summaries);
        match &result {
            Ok(summaries) => info!(
                event = "character_summaries_loaded",
                player_id = %player_id,
                characters = summaries.len(),
            ),
            Err(error) => error!(
                event = "character_summaries_failed",
                player_id = %player_id,
                error = %error,
            ),
        }
        result
    }

    pub fn scan(&self, path: impl AsRef<Path>) -> Result<ScanResponse, AppError> {
        let path = path.as_ref();
        let result = scanner::scan_url(
            path,
            &self.config.scan_log_paths,
            &self.config.resources_url,
        )
        .map(|url| ScanResponse {
            success: true,
            url,
            error: None,
            error_key: None,
        });
        match &result {
            Ok(_) => info!(
                event = "scan_completed",
                path = %path.display(),
            ),
            Err(error) => error!(
                event = "scan_failed",
                error = %error,
                path = %path.display(),
            ),
        }
        result
    }

    /// 원격 locale을 우선하고 내장 요청 언어, 내장 한국어 순으로 fallback합니다.
    ///
    /// locale 준비 실패는 기존 locale을 유지하는 비치명적 오류로 처리합니다.
    pub async fn prepare_locale(&self, lang: &str) {
        let lang = if lang.trim().is_empty() {
            self.config.language.as_str()
        } else {
            lang.trim()
        };
        let locale = match self.tracker.fetch_gacha_locale(lang).await {
            Ok(locale) => Ok(locale),
            Err(error) => {
                debug!(event = "locale_remote_fallback", lang = %lang, error = %error);
                tracker::load_local_gacha_locale(lang)
                    .or_else(|_| tracker::load_local_gacha_locale("ko"))
            }
        };

        match locale {
            Ok(locale) => {
                *self.locale.write().expect("locale lock poisoned") = Some(locale);
            }
            Err(error) => warn!(
                event = "locale_prepare_failed",
                error = %error,
                lang = %lang,
            ),
        }
    }

    /// 수집 데이터를 저장하고 해당 플레이어의 전체 통계를 반환합니다.
    ///
    /// # Errors
    ///
    /// 플레이어 ID나 기록이 없거나 저장소 처리에 실패하면 [`AppError`]를 반환합니다.
    pub fn upload(&self, fetch_result: FetchResult) -> Result<StatsResponse, AppError> {
        let player_id = fetch_result.payload.player_id.trim().to_string();
        let total_records = count_records(&fetch_result.records);
        let result = self
            .save_fetch_result(fetch_result)
            .and_then(|_| self.load_player_stats(&player_id));
        match &result {
            Ok(_) => info!(
                event = "upload_completed",
                player_id = %player_id,
                records = total_records,
            ),
            Err(error) => error!(
                event = "upload_failed",
                error = %error,
                player_id = %player_id,
                records = total_records,
            ),
        }
        result
    }

    fn save_fetch_result(&self, fetch_result: FetchResult) -> Result<(), AppError> {
        let player_id = fetch_result.payload.player_id.trim();
        if player_id.is_empty() {
            return Err(AppError::MissingPlayerId);
        }
        if fetch_result.records.is_empty() {
            return Err(AppError::EmptyUploadData);
        }

        for gacha_type in &self.config.gacha_types {
            let records = fetch_result
                .records
                .get(&gacha_type.key)
                .cloned()
                .unwrap_or_default();
            self.store
                .save_gacha_records(player_id, &gacha_type.key, &records)?;
        }
        Ok(())
    }

    pub fn get_stats(&self, player_id: impl AsRef<str>) -> Result<StatsResponse, AppError> {
        let player_id = player_id.as_ref();
        let result = self.load_player_stats(player_id);
        if let Err(error) = &result {
            error!(event = "stats_load_failed", player_id = %player_id, error = %error);
        }
        result
    }

    fn load_player_stats(&self, player_id: &str) -> Result<StatsResponse, AppError> {
        let player_id = player_id.trim();
        if player_id.is_empty() {
            return Err(AppError::MissingPlayerId);
        }
        if !self.store.has_player(player_id) {
            return Err(AppError::PlayerNotFound);
        }

        let mut stats = Vec::with_capacity(self.config.gacha_types.len());
        for gacha_type in &self.config.gacha_types {
            let records = self.store.get_gacha_records(player_id, &gacha_type.key)?;
            let gacha_type = self.localized_gacha_type(gacha_type);
            stats.push(self.calc.calc(&records, &gacha_type));
        }

        let character_summaries = character_summaries(
            &stats,
            &self.resource_types().character,
            self.config.astrite_per_pull,
        );
        let response = StatsResponse {
            success: true,
            player_id: player_id.to_string(),
            stats,
            character_summaries,
            error: None,
            error_key: None,
        };
        Ok(response)
    }

    /// URL에서 모든 배너 기록을 수집해 저장하고 최신 통계를 반환합니다.
    ///
    /// 수집한 URL의 언어 코드가 있으면 통계 계산 전에 locale도 갱신합니다.
    ///
    /// # Errors
    ///
    /// URL 검증, API 조회 또는 저장소 처리에 실패하면 [`AppError`]를 반환합니다.
    pub async fn track_url(&self, url: impl AsRef<str>) -> Result<StatsResponse, AppError> {
        let result = match self.fetch_and_save_inner(url.as_ref()).await {
            Ok(fetch_result) => self.load_player_stats(&fetch_result.payload.player_id),
            Err(error) => Err(error),
        };
        match &result {
            Ok(response) => info!(
                event = "track_url_completed",
                player_id = %response.player_id,
            ),
            Err(error) => error!(event = "track_url_failed", error = %error),
        }
        result
    }

    /// URL에서 모든 배너 기록을 수집해 저장하고 원본 수집 결과를 반환합니다.
    ///
    /// # Errors
    ///
    /// URL 검증, API 조회 또는 저장소 처리에 실패하면 [`AppError`]를 반환합니다.
    pub async fn fetch_and_save(&self, target_url: &str) -> Result<FetchResult, AppError> {
        let result = self.fetch_and_save_inner(target_url).await;
        match &result {
            Ok(fetch_result) => info!(
                event = "fetch_completed",
                player_id = %fetch_result.payload.player_id,
                records = count_records(&fetch_result.records),
            ),
            Err(error) => error!(event = "fetch_failed", error = %error),
        }
        result
    }

    async fn fetch_and_save_inner(&self, target_url: &str) -> Result<FetchResult, AppError> {
        let target_url = target_url.trim().replace('\\', "");
        if target_url.is_empty() {
            return Err(AppError::MissingUrl);
        }
        let payload = self.tracker.parse_payload_from_url(&target_url)?;
        if !payload.language_code.trim().is_empty() {
            self.prepare_locale(&payload.language_code).await;
        }
        let fetch_result = self
            .tracker
            .fetch_all_records(payload, &self.config.gacha_types)
            .await?;
        if fetch_result.records.is_empty() {
            return Err(AppError::InvalidGachaUrl);
        }
        self.save_fetch_result(fetch_result.clone())?;
        Ok(fetch_result)
    }

    pub fn export_report(
        &self,
        player_id: &str,
        format: ReportFormat,
        lang: &str,
    ) -> Result<Vec<u8>, AppError> {
        let stats = self.load_player_stats(player_id)?;
        if stats.stats.is_empty() {
            return Err(AppError::NoValidRecords);
        }
        let result = reporter::export(
            &self.config,
            &ReportData {
                player_id: stats.player_id,
                stats: stats.stats,
            },
            format,
            lang,
        );
        match &result {
            Ok(content) => info!(
                event = "report_exported",
                player_id = %player_id,
                format = %format.extension(),
                bytes = content.len(),
            ),
            Err(error) => error!(
                event = "report_export_failed",
                error = %error,
                player_id = %player_id,
                format = %format.extension(),
            ),
        }
        result
    }

    pub fn export_backup(&self) -> Result<Vec<u8>, AppError> {
        let result = self.store.export_backup();
        match &result {
            Ok(content) => info!(event = "backup_exported", bytes = content.len()),
            Err(error) => error!(event = "backup_export_failed", error = %error),
        }
        result
    }

    pub fn merge_backup(&self, input: impl AsRef<Path>) -> Result<(), AppError> {
        let input = input.as_ref();
        let result = self.store.merge_backup(input);
        match &result {
            Ok(_) => info!(
                event = "backup_merged",
                path = %input.display(),
            ),
            Err(error) => error!(
                event = "backup_merge_failed",
                error = %error,
                path = %input.display(),
            ),
        }
        result
    }

    /// 현재 형식과 배너별 map만 있는 legacy 형식의 수집 파일을 모두 읽습니다.
    ///
    /// legacy 파일은 `<player-id>-...` 파일명의 첫 구간을 플레이어 ID로 사용하며, 구간이
    /// 없으면 전체 file stem을 사용합니다.
    ///
    /// # Errors
    ///
    /// 파일 읽기나 두 형식의 JSON 역직렬화에 실패하면 [`AppError`]를 반환합니다.
    pub fn load_fetch_result_file(&self, path: impl AsRef<Path>) -> Result<FetchResult, AppError> {
        let path = path.as_ref();
        let result = self.load_fetch_result_file_inner(path);
        if let Err(error) = &result {
            error!(event = "fetch_result_file_load_failed", path = %path.display(), error = %error);
        }
        result
    }

    fn load_fetch_result_file_inner(&self, path: &Path) -> Result<FetchResult, AppError> {
        let bytes = fs::read(path)?;
        if let Ok(fetch_result) = serde_json::from_slice::<FetchResult>(&bytes) {
            if !fetch_result.records.is_empty() {
                debug!(
                    event = "fetch_result_file_loaded",
                    path = %path.display(),
                    records = count_records(&fetch_result.records),
                );
                return Ok(fetch_result);
            }
        }
        let records = serde_json::from_slice::<BTreeMap<String, Vec<Record>>>(&bytes)?;
        let player_id = path
            .file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or("offline")
            .split_once('-')
            .map(|(id, _)| id)
            .unwrap_or_else(|| {
                path.file_stem()
                    .and_then(|name| name.to_str())
                    .unwrap_or("offline")
            })
            .to_string();
        let fetch_result = FetchResult {
            payload: Payload {
                player_id,
                ..Default::default()
            },
            records,
        };
        debug!(
            event = "legacy_fetch_result_file_loaded",
            path = %path.display(),
            records = count_records(&fetch_result.records),
        );
        Ok(fetch_result)
    }

    fn localized_gacha_type(&self, gacha_type: &GachaType) -> GachaType {
        let mut item = gacha_type.clone();
        if let Some(name) = self
            .locale
            .read()
            .expect("locale lock poisoned")
            .as_ref()
            .and_then(|locale| locale.select_list.get(&item.key))
            .cloned()
        {
            item.name = name;
            return item;
        }
        if let Ok(locale) = tracker::load_local_gacha_locale(&self.config.language) {
            if let Some(name) = locale.select_list.get(&item.key).cloned() {
                item.name = name;
            }
        }
        item
    }
}

fn count_records(records: &BTreeMap<String, Vec<Record>>) -> usize {
    records.values().map(Vec::len).sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn stats_responses_include_server_character_summaries() {
        let db_path = std::env::temp_dir().join(format!(
            "wuwa-character-summary-{}-{}.redb",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let service = Service::new(Config {
            db_path: db_path.clone(),
            astrite_per_pull: 200,
            ..Default::default()
        })
        .unwrap();
        let response = service
            .upload(FetchResult {
                payload: Payload {
                    player_id: "summary-test".to_string(),
                    ..Default::default()
                },
                records: BTreeMap::from([(
                    "characterEvent".to_string(),
                    vec![Record {
                        resource_id: 100,
                        quality_level: 5,
                        resource_type: service.resource_types().character,
                        name: "Jiyan".to_string(),
                        time: "2026-01-01".to_string(),
                        ..Default::default()
                    }],
                )]),
            })
            .unwrap();
        assert_eq!(response.character_summaries.len(), 1);
        assert_eq!(response.character_summaries[0].spent_astrite, 200);
        let loaded = service.get_stats("summary-test").unwrap();
        assert_eq!(loaded.character_summaries, response.character_summaries);
        assert_eq!(
            service.character_summaries("summary-test").unwrap(),
            response.character_summaries
        );
        let json = serde_json::to_value(&loaded).unwrap();
        assert_eq!(json["characterSummaries"][0]["resourceId"], 100);
        let decoded: StatsResponse = serde_json::from_value(json).unwrap();
        assert_eq!(decoded.character_summaries, response.character_summaries);
        drop(service);
        fs::remove_file(db_path).unwrap();
    }
}
