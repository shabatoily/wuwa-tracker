use crate::{
    api,
    i18n::{I18n, Locale},
    types::{
        character_summaries, CharacterSummary, FiveStarRecord, LuckScoreThreshold, Record, Stats,
        StatsResponse,
    },
};
use leptos::prelude::*;
use serde_json::Value;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::{Event, HtmlInputElement};

const ASTRITE_PER_PULL: usize = 160;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    Dashboard,
    Characters,
}

#[derive(Clone, Copy)]
struct AppState {
    i18n: I18n,
    page: RwSignal<Page>,
    selected_character_id: RwSignal<Option<i32>>,
    scan_path: RwSignal<String>,
    url: RwSignal<String>,
    loading: RwSignal<bool>,
    scanning: RwSignal<bool>,
    error: RwSignal<String>,
    success: RwSignal<String>,
    active_player: RwSignal<String>,
    players: RwSignal<Vec<String>>,
    stats: RwSignal<Vec<Stats>>,
    // 같은 배너의 새 데이터도 `<For>` child를 다시 생성하도록 명시적인 revision을 key에 포함합니다.
    stats_revision: RwSignal<u64>,
    thresholds: RwSignal<Vec<LuckScoreThreshold>>,
    character_resource_type: RwSignal<String>,
}

impl AppState {
    fn new() -> Self {
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

    async fn initialize(self) {
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

    async fn select_player(self, player_id: String) {
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

    async fn scan(self) {
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

    async fn track(self) {
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

    async fn upload(self, data: Value, filename: String) {
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

    async fn export(self, format: &'static str) {
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

#[component]
pub fn App() -> impl IntoView {
    let state = AppState::new();
    spawn_local(async move { state.initialize().await });

    view! {
        <main class="max-w-7xl mx-auto px-6 md:px-12 py-10 md:py-16">
            <Header state />
            <ControlPanel state />
            <PageTabs state />
            <Show
                when=move || !state.stats.get().is_empty()
                fallback=move || view! {
                    <Show when=move || !state.loading.get()>
                        <div class="glass-card p-12 text-center text-slate-500 border-dashed">
                            <span class="text-4xl block mb-3">"◇"</span>
                            <p class="text-base font-bold text-slate-300 mb-1">
                                {move || state.i18n.text("app.no_data")}
                            </p>
                            <p class="text-xs">{move || state.i18n.text("app.no_data_desc")}</p>
                        </div>
                    </Show>
                }
            >
                {move || match state.page.get() {
                    Page::Dashboard => view! {
                        <div class="space-y-16">
                            <For
                                each=move || {
                                    let locale = state.i18n.locale();
                                    let revision = state.stats_revision.get();
                                    state.stats.get().into_iter().map(|stat| (locale, revision, stat)).collect::<Vec<_>>()
                                }
                                key=|(locale, revision, stat)| (*locale, *revision, stat.gacha_type)
                                children=move |(_, _, stat)| view! { <GachaReport stat state /> }
                            />
                        </div>
                    }.into_any(),
                    Page::Characters => view! { <CharactersPage state /> }.into_any(),
                }}
            </Show>
        </main>
    }
}

#[component]
fn PageTabs(state: AppState) -> impl IntoView {
    view! {
        <nav class="flex gap-2 mb-10">
            <button
                class=move || page_tab_class(state.page.get() == Page::Dashboard)
                on:click=move |_| state.page.set(Page::Dashboard)
            >
                {move || state.i18n.text("nav.dashboard")}
            </button>
            <button
                class=move || page_tab_class(state.page.get() == Page::Characters)
                on:click=move |_| {
                    state.page.set(Page::Characters);
                    state.selected_character_id.set(None);
                }
            >
                {move || state.i18n.text("nav.characters")}
            </button>
        </nav>
    }
}

#[component]
fn Header(state: AppState) -> impl IntoView {
    view! {
        <header class="flex flex-col md:flex-row justify-between items-start md:items-center gap-6 mb-8 border-b border-slate-800 pb-6">
            <div>
                <div class="flex items-center gap-2 mb-1">
                    <span class="inline-flex h-3 w-3 rounded-full bg-blue-500"></span>
                    <span class="text-xs font-semibold tracking-wider text-blue-400 uppercase">"Wuthering Waves"</span>
                </div>
                <h1 class="text-3xl md:text-4xl font-extrabold tracking-tight">
                    {move || state.i18n.text("header.title")}
                </h1>
            </div>
            <div class="flex flex-col sm:flex-row items-start sm:items-center gap-4">
                <div class="flex items-center bg-slate-950/60 p-1 rounded-lg border border-slate-800/80">
                    <button
                        class=move || locale_button_class(state.i18n.locale() == Locale::Ko)
                        on:click=move |_| state.i18n.set_locale(Locale::Ko)
                    >"KO"</button>
                    <button
                        class=move || locale_button_class(state.i18n.locale() == Locale::En)
                        on:click=move |_| state.i18n.set_locale(Locale::En)
                    >"EN"</button>
                </div>
                <div class="text-[11px] text-slate-500 bg-slate-900/40 p-2.5 rounded-lg border border-slate-800">
                    <p class="mt-0.5">{move || state.i18n.text("header.timezone_warning")}</p>
                </div>
            </div>
        </header>
    }
}

#[component]
fn ControlPanel(state: AppState) -> impl IntoView {
    let file_input: NodeRef<leptos::html::Input> = NodeRef::new();
    let handle_file = move |event: Event| {
        let input = event
            .target()
            .and_then(|target| target.dyn_into::<HtmlInputElement>().ok());
        let Some(input) = input else { return };
        let Some(file) = input.files().and_then(|files| files.get(0)) else {
            return;
        };
        let filename = file.name();
        input.set_value("");
        spawn_local(async move {
            match JsFuture::from(file.text()).await {
                Ok(value) => match value
                    .as_string()
                    .and_then(|text| serde_json::from_str(&text).ok())
                {
                    Some(data) => state.upload(data, filename).await,
                    None => {
                        state.error.set(state.i18n.text("control.invalid_json"));
                        state.success.set(String::new());
                    }
                },
                Err(_) => state.error.set(state.i18n.text("control.invalid_json")),
            }
        });
    };

    view! {
        <section class="glass-card p-8 mb-10 relative overflow-hidden">
            <div class="absolute top-0 left-0 right-0 h-[3px] bg-blue-500"></div>
            <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
                <div class="lg:col-span-2">
                    <h2 class="text-sm font-bold text-slate-300 uppercase tracking-wider mb-2">
                        {move || state.i18n.text("control.title")}
                    </h2>
                    <div class="flex flex-col gap-2">
                        <div class="flex flex-col sm:flex-row gap-2">
                            <input
                                type="text"
                                class="flex-1 bg-slate-950/80 border border-slate-800 rounded-xl px-4 py-3 text-sm focus:outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500 transition-colors text-slate-200"
                                placeholder=move || state.i18n.text("control.scan_path_placeholder")
                                prop:value=move || state.scan_path.get()
                                prop:disabled=move || state.loading.get()
                                on:input=move |event| state.scan_path.set(event_target_value(&event))
                            />
                            <button
                                class="sm:w-32 bg-slate-850 hover:bg-slate-750 active:bg-slate-900 text-slate-200 font-bold text-sm px-5 py-3 rounded-xl transition-all border border-slate-700 active:scale-95 disabled:opacity-50 whitespace-nowrap"
                                prop:disabled=move || state.loading.get()
                                on:click=move |_| spawn_local(async move { state.scan().await })
                            >
                                {move || if state.scanning.get() {
                                    state.i18n.text("control.scan_btn_loading")
                                } else {
                                    state.i18n.text("control.scan_btn")
                                }}
                            </button>
                        </div>
                        <input
                            type="text"
                            class="flex-1 bg-slate-950/80 border border-slate-800 rounded-xl px-4 py-3 text-sm focus:outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500 transition-colors text-slate-200"
                            placeholder=move || state.i18n.text("control.url_placeholder")
                            prop:value=move || state.url.get()
                            prop:disabled=move || state.loading.get()
                            on:input=move |event| state.url.set(event_target_value(&event))
                        />
                        <div class="flex gap-2">
                            <button
                                class="flex-1 sm:flex-none bg-blue-600 hover:bg-blue-500 active:bg-blue-700 text-white font-bold text-sm px-6 py-3 rounded-xl transition-all shadow-md active:scale-95 disabled:opacity-50 whitespace-nowrap"
                                prop:disabled=move || state.loading.get()
                                on:click=move |_| spawn_local(async move { state.track().await })
                            >
                                {move || if state.loading.get() {
                                    state.i18n.text("control.tracking_btn_loading")
                                } else {
                                    state.i18n.text("control.tracking_btn")
                                }}
                            </button>
                            <input node_ref=file_input type="file" accept=".json" class="hidden" on:change=handle_file />
                            <button
                                class="flex-1 sm:flex-none bg-slate-850 hover:bg-slate-750 active:bg-slate-900 text-slate-200 font-bold text-sm px-5 py-3 rounded-xl transition-all border border-slate-700 active:scale-95 disabled:opacity-50 whitespace-nowrap"
                                prop:disabled=move || state.loading.get()
                                on:click=move |_| {
                                    if let Some(input) = file_input.get() {
                                        input.click();
                                    }
                                }
                            >{move || state.i18n.text("control.upload_btn")}</button>
                        </div>
                    </div>
                </div>
                <div>
                    <h2 class="text-sm font-bold text-slate-300 uppercase tracking-wider mb-2">
                        {move || state.i18n.format(
                            "control.saved_players_title",
                            &[("count", state.players.get().len().to_string())],
                        )}
                    </h2>
                    <Show
                        when=move || !state.players.get().is_empty()
                        fallback=move || view! {
                            <p class="text-xs text-slate-500 italic py-3">
                                {move || state.i18n.text("control.no_players")}
                            </p>
                        }
                    >
                        <div class="flex flex-wrap gap-2 max-h-[110px] overflow-y-auto custom-scrollbar p-1">
                            <For
                                each=move || state.players.get()
                                key=|player| player.clone()
                                children=move |player| {
                                    let selected = player.clone();
                                    view! {
                                        <button
                                            class=move || player_button_class(state.active_player.get() == player)
                                            prop:disabled=move || state.loading.get()
                                            on:click=move |_| {
                                                let player = selected.clone();
                                                spawn_local(async move { state.select_player(player).await });
                                            }
                                        >{player.clone()}</button>
                                    }
                                }
                            />
                        </div>
                    </Show>
                </div>
            </div>
            <Show when=move || !state.active_player.get().is_empty()>
                <div class="mt-6 pt-4 border-t border-slate-800/60 flex flex-wrap items-center gap-3">
                    <span class="text-xs text-slate-500 font-bold uppercase tracking-wider">
                        {move || format!("{}:", state.i18n.text("control.export_report"))}
                    </span>
                    <div class="flex gap-2">
                        <ExportButton state format="html" label="HTML" class="text-blue-400 border-blue-500/30" />
                        <ExportButton state format="json" label="JSON" class="text-amber-400 border-amber-500/30" />
                        <ExportButton state format="csv" label="CSV" class="text-emerald-400 border-emerald-500/30" />
                    </div>
                </div>
            </Show>
            <Show when=move || !state.error.get().is_empty()>
                <div class="mt-4 bg-rose-500/10 border border-rose-500/20 text-rose-400 rounded-xl p-3.5 text-sm">
                    {move || state.error.get()}
                </div>
            </Show>
            <Show when=move || !state.success.get().is_empty()>
                <div class="mt-4 bg-emerald-500/10 border border-emerald-500/20 text-emerald-400 rounded-xl p-3.5 text-sm">
                    {move || state.success.get()}
                </div>
            </Show>
        </section>
    }
}

#[component]
fn ExportButton(
    state: AppState,
    format: &'static str,
    label: &'static str,
    class: &'static str,
) -> impl IntoView {
    let class = format!(
        "text-xs px-3 py-1.5 font-bold rounded-lg border bg-slate-900/60 {class} hover:bg-blue-600/10 transition-all active:scale-95"
    );
    view! {
        <button class=class on:click=move |_| spawn_local(async move { state.export(format).await })>
            {label}
        </button>
    }
}

#[component]
fn CharactersPage(state: AppState) -> impl IntoView {
    view! {
        {move || {
            let summaries = character_summaries(
                &state.stats.get(),
                &state.character_resource_type.get(),
                ASTRITE_PER_PULL,
            );
            match state.selected_character_id.get() {
                Some(resource_id) => summaries
                    .iter()
                    .find(|summary| summary.resource_id == resource_id)
                    .cloned()
                    .map(|summary| view! { <CharacterDetail state summary /> }.into_any())
                    .unwrap_or_else(|| view! { <CharacterList state summaries /> }.into_any()),
                None => view! { <CharacterList state summaries /> }.into_any(),
            }
        }}
    }
}

#[component]
fn CharacterList(state: AppState, summaries: Vec<CharacterSummary>) -> impl IntoView {
    if summaries.is_empty() {
        return view! {
            <section class="glass-card p-12 text-center text-slate-500 border-dashed">
                <span class="text-4xl block mb-3">"◇"</span>
                <p class="text-base font-bold text-slate-300 mb-1">
                    {state.i18n.text("characters.empty")}
                </p>
                <p class="text-xs">{state.i18n.text("characters.empty_desc")}</p>
            </section>
        }
        .into_any();
    }

    view! {
        <section class="space-y-6">
            <div class="flex flex-col sm:flex-row sm:items-start sm:justify-between gap-3">
                <div>
                    <h2 class="text-2xl font-extrabold text-slate-100">{state.i18n.text("characters.title")}</h2>
                    <p class="text-xs text-slate-500 mt-1">{state.i18n.text("characters.subtitle")}</p>
                </div>
                <span class="inline-flex h-6 items-center text-xs font-semibold text-slate-400 bg-slate-900/60 px-2.5 rounded-md border border-slate-800 whitespace-nowrap">
                    {state.i18n.format("characters.count", &[("count", summaries.len().to_string())])}
                </span>
            </div>
            <div class="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-4">
                <For
                    each=move || summaries.clone()
                    key=|summary| summary.resource_id
                    children=move |summary| {
                        let resource_id = summary.resource_id;
                        view! {
                            <button
                                class="text-left bg-slate-950/45 hover:bg-slate-900/70 border border-slate-800/80 rounded-xl p-5 transition-colors active:scale-[0.99]"
                                on:click=move |_| state.selected_character_id.set(Some(resource_id))
                            >
                                <div class="flex items-start justify-between gap-3 mb-4">
                                    <div>
                                        <p class="text-lg font-extrabold text-amber-400">{summary.name}</p>
                                        <p class="text-xs text-slate-500 mt-1">{summary.resource_type}</p>
                                    </div>
                                    <span class="px-2 py-0.5 text-xs rounded border bg-amber-500/10 text-amber-400 border-amber-500/20">
                                        {format!("{}★", summary.quality_level)}
                                    </span>
                                </div>
                                <div class="grid grid-cols-2 gap-3">
                                    <CharacterMetric label=state.i18n.text("characters.copies") value=summary.copies.to_string() />
                                    <CharacterMetric label=state.i18n.text("characters.spent_astrite") value=format_number(summary.spent_astrite) />
                                    <CharacterMetric label=state.i18n.text("characters.banner_count") value=summary.banner_count.to_string() />
                                </div>
                            </button>
                        }
                    }
                />
            </div>
        </section>
    }
    .into_any()
}

#[component]
fn CharacterDetail(state: AppState, summary: CharacterSummary) -> impl IntoView {
    view! {
        <section class="glass-card p-8 md:p-10">
            <button
                class="mb-6 text-xs px-3 py-1.5 font-bold rounded-lg border bg-slate-900/60 text-slate-400 border-slate-800 hover:border-slate-700"
                on:click=move |_| state.selected_character_id.set(None)
            >
                {state.i18n.text("characters.back")}
            </button>
            <div class="flex flex-col sm:flex-row justify-between items-start sm:items-center gap-4 mb-8 border-b border-slate-800/60 pb-5">
                <div>
                    <h2 class="text-3xl font-extrabold text-amber-400">{summary.name.clone()}</h2>
                    <p class="text-xs text-slate-500 mt-1">{summary.resource_type.clone()}</p>
                </div>
                <span class="px-3 py-1 text-sm rounded border bg-amber-500/10 text-amber-400 border-amber-500/20">
                    {format!("{}★", summary.quality_level)}
                </span>
            </div>
            <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
                <Metric label=state.i18n.text("characters.copies") value=summary.copies.to_string() class="text-slate-100" />
                <Metric label=state.i18n.text("characters.spent_astrite") value=format_number(summary.spent_astrite) class="text-sky-300" />
                <Metric label=state.i18n.text("characters.banner_count") value=summary.banner_count.to_string() class="text-emerald-400" />
            </div>
        </section>
    }
}

#[component]
fn CharacterMetric(label: String, value: String) -> impl IntoView {
    view! {
        <div>
            <p class="text-[11px] text-slate-500 mb-1">{label}</p>
            <p class="text-sm font-extrabold text-slate-200">{value}</p>
        </div>
    }
}

#[component]
fn GachaReport(stat: Stats, state: AppState) -> impl IntoView {
    let has_pick_up = stat.has_pick_up();
    let average_label = if matches!(stat.gacha_type, 1 | 8 | 10 | 12) {
        "report.my_avg_pickup_pulls"
    } else {
        "report.my_avg_pulls"
    };
    let luck_state = luck_state(stat.luck_score, &state.thresholds.get_untracked());
    let luck_class = luck_text_class(&luck_state);
    let luck_panel_class = if has_pick_up {
        luck_panel_class(&luck_state)
    } else {
        "bg-slate-900/40 border-slate-800/50"
    };
    let pity_width = ((stat.current_pity5 as f64 / 80.0) * 100.0).min(100.0);
    let five_star_count = stat.five_stars.len();
    let actual_rate_class = if stat.actual_rate >= stat.base_rate {
        "text-emerald-400 font-extrabold"
    } else {
        "text-rose-400 font-extrabold"
    };
    let five_stars = stat.five_stars.clone();
    let records = stat.records.clone();

    view! {
        <article class="glass-card p-8 md:p-10 relative overflow-hidden">
            <div class="flex flex-col sm:flex-row justify-between items-start sm:items-center gap-4 mb-6 border-b border-slate-800/60 pb-4">
                <h3 class="text-xl md:text-2xl font-bold text-slate-100 flex items-center gap-2">
                    <span class="w-2 h-6 bg-blue-500 rounded"></span>
                    {if stat.gacha_name.is_empty() { state.i18n.text("report.loading") } else { stat.gacha_name.clone() }}
                </h3>
                <span class="text-xs font-semibold text-slate-500 bg-slate-900 px-3 py-1 rounded-full border border-slate-800">
                    {state.i18n.format("report.type_code", &[("code", stat.gacha_type.to_string())])}
                </span>
            </div>
            <div class="grid grid-cols-1 lg:grid-cols-3 gap-8">
                <div class="lg:col-span-2 space-y-6">
                    <div class="grid grid-cols-1 sm:grid-cols-3 gap-4">
                        <Metric label=state.i18n.text("report.total_pulls") value=stat.total_pulls.to_string() class="text-slate-100" />
                        <Metric label=state.i18n.text("report.total_astrite") value=format_number(stat.total_astrite) class="text-sky-300" />
                        <div class="bg-slate-900/40 p-5 rounded-xl border border-slate-800/50">
                            <p class="text-xs text-slate-500 mb-1">{state.i18n.text("report.current_pity")}</p>
                            <p class="text-2xl font-extrabold text-amber-400">
                                {stat.current_pity5}<span class="text-xs text-slate-500 font-normal">{state.i18n.text("report.pity_suffix")}</span>
                            </p>
                            <div class="mt-2.5 w-full bg-slate-950/80 rounded-full h-2 overflow-hidden border border-slate-800/80">
                                <div class="bg-amber-500 h-full rounded-full" style=format!("width: {pity_width}%")></div>
                            </div>
                        </div>
                        <div class=format!("p-5 rounded-xl border {luck_panel_class}")>
                            <p class="text-xs text-slate-500 mb-1">{state.i18n.text("report.luck_score")}</p>
                            <p class="text-2xl font-extrabold">
                                {if has_pick_up {
                                    format!("{} ({:.0}%)", state.i18n.text(&format!("report.luck_score_state.{luck_state}")), stat.luck_score)
                                } else {
                                    state.i18n.text("report.luck_score_unknown")
                                }}
                            </p>
                            <span class=luck_class></span>
                        </div>
                    </div>
                    <div class="bg-slate-950/50 rounded-xl border border-slate-850 p-5">
                        <h4 class="text-xs font-bold text-blue-400 uppercase tracking-wider mb-3">
                            {state.i18n.text("report.efficiency_analysis")}
                        </h4>
                        <div class="grid grid-cols-1 sm:grid-cols-3 gap-4 text-sm">
                            <Efficiency label=state.i18n.text(average_label) value=if has_pick_up { format!("{:.1}{}", stat.avg_pulls, state.i18n.text("report.avg_pulls_suffix")) } else { state.i18n.text("report.no_avg_pulls") } />
                            <Efficiency label=state.i18n.text("report.expected_avg_pulls") value=format!("{}{}", stat.expected_pulls, state.i18n.text("report.avg_pulls_suffix")) />
                            <div class="flex justify-between sm:flex-col sm:justify-start gap-1">
                                <span class="text-slate-500 text-xs">{state.i18n.text("report.actual_rate_vs_base")}</span>
                                <span class=actual_rate_class>{format!("{:.2}% / {:.1}%", stat.actual_rate, stat.base_rate)}</span>
                            </div>
                        </div>
                    </div>
                    <div>
                        <h4 class="text-sm font-bold text-slate-400 mb-3 flex items-center gap-2">
                            <span class="w-1.5 h-1.5 bg-amber-500 rounded-full"></span>
                            {state.i18n.format("report.history_title", &[("count", five_star_count.to_string())])}
                        </h4>
                        <FiveStarHistory records=five_stars i18n=state.i18n />
                    </div>
                </div>
                <FullHistory records i18n=state.i18n />
            </div>
        </article>
    }
}

#[component]
fn Metric(label: String, value: String, class: &'static str) -> impl IntoView {
    view! {
        <div class="bg-slate-900/40 p-5 rounded-xl border border-slate-800/50">
            <p class="text-xs text-slate-500 mb-1">{label}</p>
            <p class=format!("text-2xl font-extrabold {class}")>{value}</p>
        </div>
    }
}

#[component]
fn Efficiency(label: String, value: String) -> impl IntoView {
    view! {
        <div class="flex justify-between sm:flex-col sm:justify-start gap-1">
            <span class="text-slate-500 text-xs">{label}</span>
            <span class="font-bold text-slate-200">{value}</span>
        </div>
    }
}

#[component]
fn FiveStarHistory(records: Vec<FiveStarRecord>, i18n: I18n) -> impl IntoView {
    if records.is_empty() {
        return view! {
            <div class="bg-slate-950/20 border border-dashed border-slate-800 rounded-xl p-6 text-center text-slate-500 text-sm">
                {i18n.text("report.history_empty")}
            </div>
        }
        .into_any();
    }

    view! {
        <div class="overflow-x-auto border border-slate-800/80 rounded-xl bg-slate-950/30">
            <table class="min-w-full divide-y divide-slate-800">
                <thead class="bg-slate-900/60"><tr>
                    <TableHead text=i18n.text("table.name") />
                    <TableHead text=i18n.text("table.pity") />
                    <TableHead text=i18n.text("table.time") />
                    <TableHead text=i18n.text("table.category") />
                </tr></thead>
                <tbody class="divide-y divide-slate-800/60 bg-transparent">
                    {records.into_iter().map(|record| view! {
                        <tr class="hover:bg-slate-900/20 transition-colors">
                            <td class="px-4 py-3.5 whitespace-nowrap text-sm font-bold text-amber-400">{record.name}</td>
                            <td class="px-4 py-3.5 whitespace-nowrap text-sm text-slate-200">
                                <span class="font-extrabold text-amber-500 bg-amber-500/10 px-2 py-0.5 rounded border border-amber-500/20">
                                    {i18n.format("table.pity_suffix", &[("count", record.pity.to_string())])}
                                </span>
                            </td>
                            <td class="px-4 py-3.5 whitespace-nowrap text-xs text-slate-400">{record.time}</td>
                            <td class="px-4 py-3.5 whitespace-nowrap text-sm">
                                <span class=if record.is_pick_up { "px-2.5 py-0.5 text-xs rounded-full bg-emerald-500/10 text-emerald-400 border border-emerald-500/20" } else { "px-2.5 py-0.5 text-xs rounded-full bg-rose-500/10 text-rose-400 border border-rose-500/20" }>
                                    {if record.is_pick_up { i18n.text("table.pickup_success") } else { i18n.text("table.pickup_fail") }}
                                </span>
                            </td>
                        </tr>
                    }).collect_view()}
                </tbody>
            </table>
        </div>
    }
    .into_any()
}

#[component]
fn FullHistory(records: Vec<Record>, i18n: I18n) -> impl IntoView {
    let count = records.len();
    if records.is_empty() {
        return view! {
            <div class="flex flex-col h-[650px] bg-slate-950/40 p-5 rounded-2xl border border-slate-900/60">
                <h4 class="text-sm font-bold text-slate-300 mb-4 pb-2.5 border-b border-slate-900/60">
                    {i18n.format("full_history.title", &[("count", count.to_string())])}
                </h4>
                <div class="p-6 text-center text-slate-500 text-xs italic">
                    {i18n.text("full_history.empty")}
                </div>
            </div>
        }
        .into_any();
    }

    view! {
        <div class="flex flex-col h-[650px] bg-slate-950/40 p-5 rounded-2xl border border-slate-900/60">
            <h4 class="text-sm font-bold text-slate-300 mb-4 pb-2.5 border-b border-slate-900/60">
                {i18n.format("full_history.title", &[("count", count.to_string())])}
            </h4>
            <div class="flex-1 overflow-y-auto custom-scrollbar pr-1">
                <table class="min-w-full divide-y divide-slate-900">
                    <thead class="bg-slate-950/80 sticky top-0 z-10"><tr>
                        <TableHead text=i18n.text("table.name") compact=true />
                        <TableHead text=i18n.text("table.rarity") compact=true />
                        <TableHead text=i18n.text("table.type") compact=true />
                    </tr></thead>
                    <tbody class="divide-y divide-slate-900/50 bg-transparent">
                        {records.into_iter().map(|record| {
                            let name_class = match record.quality_level {
                                5 => "font-bold text-amber-400",
                                4 => "font-semibold text-purple-300",
                                _ => "text-slate-300",
                            };
                            let rarity_class = match record.quality_level {
                                5 => "bg-amber-500/10 text-amber-400 border-amber-500/20",
                                4 => "bg-purple-500/10 text-purple-300 border-purple-500/20",
                                _ => "bg-slate-800 text-slate-400 border-slate-700/50",
                            };
                            view! {
                                <tr class="hover:bg-slate-900/30 transition-colors">
                                    <td class=format!("px-3 py-2.5 whitespace-nowrap text-xs {name_class}")>{record.name}</td>
                                    <td class="px-3 py-2.5 whitespace-nowrap text-[10px]">
                                        <span class=format!("px-1.5 py-0.5 inline-flex leading-3 rounded border {rarity_class}")>{format!("{}★", record.quality_level)}</span>
                                    </td>
                                    <td class="px-3 py-2.5 whitespace-nowrap text-[10px] text-slate-500">{record.resource_type}</td>
                                </tr>
                            }
                        }).collect_view()}
                    </tbody>
                </table>
            </div>
        </div>
    }
    .into_any()
}

#[component]
fn TableHead(text: String, #[prop(default = false)] compact: bool) -> impl IntoView {
    let padding = if compact { "px-3 py-2" } else { "px-4 py-3" };
    view! {
        <th class=format!("{padding} text-left text-xs font-semibold text-slate-400 uppercase tracking-wider")>{text}</th>
    }
}

fn luck_state(score: f64, thresholds: &[LuckScoreThreshold]) -> String {
    // 구간은 min_score 오름차순이라는 config 계약을 사용해 가장 높은 일치 항목을 선택합니다.
    thresholds
        .iter()
        .rfind(|threshold| score >= threshold.min_score)
        .map(|threshold| threshold.state.clone())
        .unwrap_or_else(|| "normal".to_string())
}

fn luck_text_class(state: &str) -> &'static str {
    match state {
        "worst" => "text-rose-500 font-extrabold",
        "bad" => "text-rose-300",
        "good" | "best" => "text-emerald-400",
        _ => "text-slate-300",
    }
}

fn luck_panel_class(state: &str) -> &'static str {
    match state {
        "worst" => "bg-rose-500/10 border-rose-500/30",
        "bad" => "bg-rose-500/5 border-rose-500/10",
        "good" => "bg-emerald-500/5 border-emerald-500/20",
        "best" => "bg-emerald-500/10 border-emerald-500/30",
        _ => "bg-slate-900/40 border-slate-800/50",
    }
}

fn page_tab_class(active: bool) -> &'static str {
    if active {
        "px-4 py-2 text-sm font-bold rounded-lg border bg-blue-600/20 text-blue-400 border-blue-500"
    } else {
        "px-4 py-2 text-sm font-bold rounded-lg border bg-slate-900/60 text-slate-400 border-slate-800 hover:border-slate-700"
    }
}

fn locale_button_class(active: bool) -> &'static str {
    if active {
        "px-2.5 py-1 text-[11px] font-bold rounded bg-blue-600/35 text-blue-400 border border-blue-500/30"
    } else {
        "px-2.5 py-1 text-[11px] font-bold rounded text-slate-500 border border-transparent hover:text-slate-300"
    }
}

fn player_button_class(active: bool) -> &'static str {
    if active {
        "text-xs px-3 py-1.5 font-bold rounded-lg border bg-blue-600/20 text-blue-400 border-blue-500"
    } else {
        "text-xs px-3 py-1.5 font-bold rounded-lg border bg-slate-900/60 text-slate-400 border-slate-800 hover:border-slate-700"
    }
}

fn format_number(value: usize) -> String {
    let digits = value.to_string();
    let mut formatted = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, character) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            formatted.push(',');
        }
        formatted.push(character);
    }
    formatted
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn luck_state_uses_highest_matching_threshold() {
        let thresholds = vec![
            LuckScoreThreshold {
                min_score: 0.0,
                state: "worst".to_string(),
            },
            LuckScoreThreshold {
                min_score: 95.0,
                state: "normal".to_string(),
            },
            LuckScoreThreshold {
                min_score: 115.0,
                state: "best".to_string(),
            },
        ];

        assert_eq!(luck_state(100.0, &thresholds), "normal");
    }

    #[test]
    fn format_number_groups_thousands() {
        assert_eq!(format_number(1_234_567), "1,234,567");
    }
}
