use leptos::prelude::*;

#[component]
pub fn UtilizationBar(#[prop(into)] used: i32, #[prop(into)] total: i32) -> impl IntoView {
    let pct = if total == 0 {
        0.0
    } else {
        (used as f64 / total as f64) * 100.0
    };

    let color = if pct >= 90.0 {
        "bg-red-500"
    } else if pct >= 70.0 {
        "bg-yellow-500"
    } else {
        "bg-green-500"
    };

    let width_style = format!("width: {:.1}%", pct.min(100.0));

    view! {
        <div class="space-y-1">
            <div class="flex justify-between text-xs text-slate-400">
                <span>{format!("{}/{}", used, total)}</span>
                <span>{format!("{:.1}%", pct)}</span>
            </div>
            <div class="w-full bg-slate-700 rounded-full h-2">
                <div class={format!("{} h-2 rounded-full transition-all", color)}
                     style=width_style>
                </div>
            </div>
        </div>
    }
}
