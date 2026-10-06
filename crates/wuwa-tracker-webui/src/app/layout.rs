use super::state::{AppState, Page};
use crate::i18n::Locale;
use leptos::prelude::*;

#[component]
pub(super) fn PageTabs(state: AppState) -> impl IntoView {
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
pub(super) fn Header(state: AppState) -> impl IntoView {
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
