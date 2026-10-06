use super::state::AppState;
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::{Event, HtmlInputElement};

#[component]
pub(super) fn ControlPanel(state: AppState) -> impl IntoView {
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

fn player_button_class(active: bool) -> &'static str {
    if active {
        "text-xs px-3 py-1.5 font-bold rounded-lg border bg-blue-600/20 text-blue-400 border-blue-500"
    } else {
        "text-xs px-3 py-1.5 font-bold rounded-lg border bg-slate-900/60 text-slate-400 border-slate-800 hover:border-slate-700"
    }
}
