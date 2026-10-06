mod characters;
mod controls;
mod layout;
mod report;
mod shared;
mod state;

use characters::CharactersPage;
use controls::ControlPanel;
use layout::{Header, PageTabs};
use leptos::prelude::*;
use report::GachaReport;
use state::{AppState, Page};
use wasm_bindgen_futures::spawn_local;

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
