use super::{
    shared::{format_number, Metric},
    state::AppState,
};
use crate::{
    i18n::I18n,
    types::{FiveStarRecord, LuckScoreThreshold, Record, Stats},
};
use leptos::prelude::*;

#[component]
pub(super) fn GachaReport(stat: Stats, state: AppState) -> impl IntoView {
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
}
