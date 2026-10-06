use super::{
    shared::{format_number, Metric},
    state::AppState,
};
use crate::types::{character_summaries, CharacterSummary};
use leptos::prelude::*;

const ASTRITE_PER_PULL: usize = 160;

#[component]
pub(super) fn CharactersPage(state: AppState) -> impl IntoView {
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
