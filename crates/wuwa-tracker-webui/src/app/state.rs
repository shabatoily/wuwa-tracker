use crate::{
    api,
    i18n::I18n,
    types::{LuckScoreThreshold, Stats, StatsResponse},
};
use leptos::prelude::*;
use serde_json::Value;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Page {
    Dashboard,
    Characters,
}

#[derive(Clone, Copy)]
pub(super) struct AppState {
    pub(super) i18n: I18n,
    pub(super) page: RwSignal<Page>,
    pub(super) selected_character_id: RwSignal<Option<i32>>,
    pub(super) scan_path: RwSignal<String>,
    pub(super) url: RwSignal<String>,
    pub(super) loading: RwSignal<bool>,
    pub(super) scanning: RwSignal<bool>,
    pub(super) error: RwSignal<String>,
    pub(super) success: RwSignal<String>,
    pub(super) active_player: RwSignal<String>,
    pub(super) players: RwSignal<Vec<String>>,
    pub(super) stats: RwSignal<Vec<Stats>>,
    // 같은 배너의 새 데이터도 `<For>` child를 다시 생성하도록 명시적인 revision을 key에 포함합니다.
    pub(super) stats_revision: RwSignal<u64>,
    pub(super) thresholds: RwSignal<Vec<LuckScoreThreshold>>,
    pub(super) character_resource_type: RwSignal<String>,
}

impl AppState {
    pub(super) fn new() -> Self {
        let scan_path = web_sys::window()
            .and_then(|window| window.local_storage().ok().flatten())
            .and_then(|storage| storage.get_item("scanPath").ok().flatten())
            .unwrap_or_default();
        Self {
            i18n: I18n::new(),
            page: RwSignal::new(Page::Dashboard),
            selected_character_id: RwSignal::new(None),
            scan_path: RwSignal::new(scan_path),
            url: RwSignal::new(String::new()),
            loading: RwSignal::new(false),
            scanning: RwSignal::new(false),
            error: RwSignal::new(String::new()),
            success: RwSignal::new(String::new()),
            active_player: RwSignal::new(String::new()),
            players: RwSignal::new(Vec::new()),
            stats: RwSignal::new(Vec::new()),
            stats_revision: RwSignal::new(0),
            thresholds: RwSignal::new(Vec::new()),
            character_resource_type: RwSignal::new(String::new()),
        }
    }

    fn clear_messages(self) {
        self.error.set(String::new());
        self.success.set(String::new());
    }

    fn translated_error(self, response: &StatsResponse, fallback: &str) -> String {
        response
            .error_key
            .as_deref()
            .map(|key| self.i18n.text(key))
            .or_else(|| response.error.clone())
            .unwrap_or_else(|| self.i18n.text(fallback))
    }

    fn replace_stats(self, stats: Vec<Stats>) {
        self.stats.set(stats);
        self.stats_revision
            .update(|revision| *revision = revision.wrapping_add(1));
    }

    pub(super) async fn initialize(self) {
        if let Ok(config) = api::fetch_config().await {
            if config.success {
                self.thresholds.set(config.luck_score_thresholds);
                self.character_resource_type
                    .set(config.resource_types.character);
            }
        }
        self.load_players().await;
        if let Some(player) = self.players.get_untracked().first().cloned() {
            self.select_player(player).await;
        }
    }

    async fn load_players(self) {
        if let Ok(response) = api::fetch_players().await {
            if response.success {
                self.players.set(response.players);
            }
        }
    }

    pub(super) async fn select_player(self, player_id: String) {
        self.loading.set(true);
        self.clear_messages();
        self.active_player.set(player_id.clone());
        self.selected_character_id.set(None);
        match api::fetch_stats(&player_id).await {
            Ok(response) if response.success => self.replace_stats(response.stats),
            Ok(response) => {
                self.replace_stats(Vec::new());
                self.error
                    .set(self.translated_error(&response, "app.failed_stats"));
            }
            Err(_) => {
                self.replace_stats(Vec::new());
                self.error.set(self.i18n.text("app.network_error"));
            }
        }
        self.loading.set(false);
    }

    pub(super) async fn scan(self) {
        let path = self.scan_path.get_untracked().trim().to_string();
        if path.is_empty() {
            self.error.set(self.i18n.text("app.scan_path_empty_alert"));
            self.success.set(String::new());
            return;
        }

        if let Some(storage) =
            web_sys::window().and_then(|window| window.local_storage().ok().flatten())
        {
            let _ = storage.set_item("scanPath", &path);
        }
        self.loading.set(true);
        self.scanning.set(true);
        self.clear_messages();
        match api::scan_path(&path).await {
            Ok(response) if response.success => {
                self.url.set(response.url);
                self.success.set(self.i18n.text("app.scan_success"));
            }
            Ok(response) => {
                let message = response
                    .error_key
                    .as_deref()
                    .map(|key| self.i18n.text(key))
                    .or(response.error)
                    .unwrap_or_else(|| self.i18n.text("app.failed_scan"));
                self.error.set(message);
            }
            Err(_) => self.error.set(self.i18n.text("app.network_error")),
        }
        self.scanning.set(false);
        self.loading.set(false);
    }

    pub(super) async fn track(self) {
        let url = self.url.get_untracked().trim().to_string();
        if url.is_empty() {
            self.error.set(self.i18n.text("app.url_empty_alert"));
            return;
        }

        self.loading.set(true);
        self.clear_messages();
        match api::track_url(&url).await {
            Ok(response) if response.success => {
                let player_id = response.player_id;
                self.active_player.set(player_id.clone());
                self.replace_stats(response.stats);
                self.success.set(
                    self.i18n
                        .format("app.track_success", &[("playerId", player_id.clone())]),
                );
                self.url.set(String::new());
                self.load_players().await;
            }
            Ok(response) => self
                .error
                .set(self.translated_error(&response, "app.failed_scan")),
            Err(_) => self.error.set(self.i18n.text("app.network_error")),
        }
        self.loading.set(false);
    }

    pub(super) async fn upload(self, data: Value, filename: String) {
        let player_id = data
            .get("payload")
            .and_then(|payload| payload.get("playerId"))
            .and_then(Value::as_str)
            .map(str::trim)
            .unwrap_or_default();
        if player_id.is_empty() {
            self.error.set(self.i18n.text("app.failed_upload"));
            return;
        }

        self.loading.set(true);
        self.clear_messages();
        match api::upload_json(&data).await {
            Ok(response) if response.success => {
                let player_id = response.player_id;
                self.active_player.set(player_id.clone());
                self.replace_stats(response.stats);
                self.success.set(self.i18n.format(
                    "app.upload_success",
                    &[("fileName", filename), ("playerId", player_id)],
                ));
                self.load_players().await;
            }
            Ok(response) => self
                .error
                .set(self.translated_error(&response, "app.failed_upload")),
            Err(_) => self.error.set(self.i18n.text("app.failed_upload_network")),
        }
        self.loading.set(false);
    }

    pub(super) async fn export(self, format: &'static str) {
        let player_id = self.active_player.get_untracked();
        if player_id.is_empty() {
            return;
        }
        self.loading.set(true);
        self.clear_messages();
        match api::export_report(&player_id, format, self.i18n.locale().code()).await {
            Ok(()) => self.success.set(self.i18n.text("control.export_report")),
            Err(_) => self.error.set(self.i18n.text("app.network_error")),
        }
        self.loading.set(false);
    }
}
