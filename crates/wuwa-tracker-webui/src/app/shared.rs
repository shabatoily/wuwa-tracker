use leptos::prelude::*;

#[component]
pub(super) fn Metric(label: String, value: String, class: &'static str) -> impl IntoView {
    view! {
        <div class="bg-slate-900/40 p-5 rounded-xl border border-slate-800/50">
            <p class="text-xs text-slate-500 mb-1">{label}</p>
            <p class=format!("text-2xl font-extrabold {class}")>{value}</p>
        </div>
    }
}

pub(super) fn format_number(value: usize) -> String {
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
    fn format_number_groups_thousands() {
        assert_eq!(format_number(1_234_567), "1,234,567");
    }
}
