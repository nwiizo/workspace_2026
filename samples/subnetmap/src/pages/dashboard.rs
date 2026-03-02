use leptos::prelude::*;

use crate::models::alert::AlertType;
use crate::server::alerts::list_alerts;
use crate::server::audit::list_audit_logs;
use crate::server::search::get_dashboard_stats;

#[component]
pub fn DashboardPage() -> impl IntoView {
    let stats = Resource::new(|| (), |_| get_dashboard_stats());
    let alerts = Resource::new(|| (), |_| list_alerts(Some(true)));
    let recent_logs = Resource::new(|| (), |_| list_audit_logs(None, None, None, Some(10)));

    view! {
        <div>
            <h1 class="text-2xl font-bold text-white mb-6">"Dashboard"</h1>

            <Suspense fallback=|| view! { <div class="text-slate-400">"Loading..."</div> }>
                {move || stats.get().map(|result| match result {
                    Ok(s) => view! {
                        <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4 mb-8">
                            <StatCard label="Total Subnets" value=s.total_subnets.to_string() color="blue" />
                            <StatCard label="Total IPs" value=s.total_ips.to_string() color="green" />
                            <StatCard label="Assigned IPs" value=s.assigned_ips.to_string() color="purple" />
                            <StatCard label="VLANs" value=s.total_vlans.to_string() color="cyan" />
                            <StatCard label="Active Alerts" value=s.active_alerts.to_string() color="red" />
                            <StatCard label="Avg Utilization" value=format!("{:.1}%", s.avg_utilization) color="yellow" />
                        </div>
                    }.into_any(),
                    Err(e) => view! { <p class="text-red-400">{e.to_string()}</p> }.into_any(),
                })}
            </Suspense>

            <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
                <div class="bg-slate-800 border border-slate-700 rounded-xl p-5">
                    <h2 class="text-lg font-semibold text-white mb-4">"Active Alerts"</h2>
                    <Suspense fallback=|| view! { <div class="text-slate-400">"Loading..."</div> }>
                        {move || alerts.get().map(|result| match result {
                            Ok(alert_list) => {
                                if alert_list.is_empty() {
                                    view! { <p class="text-slate-500 text-sm">"No active alerts"</p> }.into_any()
                                } else {
                                    view! {
                                        <div class="space-y-2">
                                            {alert_list.into_iter().map(|alert| {
                                                let alert_type = AlertType::parse(&alert.alert_type);
                                                let class = format!("p-3 rounded-lg text-sm {}", alert_type.color_class());
                                                let at = alert.alert_type.clone();
                                                let msg = alert.message.clone();
                                                view! {
                                                    <div class=class>
                                                        <span class="font-medium">{at}</span>
                                                        ": "
                                                        {msg}
                                                    </div>
                                                }
                                            }).collect::<Vec<_>>()}
                                        </div>
                                    }.into_any()
                                }
                            },
                            Err(e) => view! { <p class="text-red-400">{e.to_string()}</p> }.into_any(),
                        })}
                    </Suspense>
                </div>

                <div class="bg-slate-800 border border-slate-700 rounded-xl p-5">
                    <div class="flex justify-between items-center mb-4">
                        <h2 class="text-lg font-semibold text-white">"Recent Changes"</h2>
                        <a href="/audit" class="text-blue-400 hover:text-blue-300 text-sm">"View all"</a>
                    </div>
                    <Suspense fallback=|| view! { <div class="text-slate-400">"Loading..."</div> }>
                        {move || recent_logs.get().map(|result| match result {
                            Ok(logs) => {
                                if logs.is_empty() {
                                    view! { <p class="text-slate-500 text-sm">"No recent changes"</p> }.into_any()
                                } else {
                                    view! {
                                        <div class="space-y-2">
                                            {logs.into_iter().map(|log| {
                                                let action = log.action.clone();
                                                let entity = log.entity_type.clone();
                                                let time = log.created_at.format("%m/%d %H:%M").to_string();
                                                view! {
                                                    <div class="flex justify-between items-center py-2 border-b border-slate-700 last:border-0">
                                                        <div>
                                                            <span class="text-slate-300 text-sm">{action}</span>
                                                            " "
                                                            <span class="text-slate-500 text-xs">{entity}</span>
                                                        </div>
                                                        <span class="text-slate-500 text-xs">{time}</span>
                                                    </div>
                                                }
                                            }).collect::<Vec<_>>()}
                                        </div>
                                    }.into_any()
                                }
                            },
                            Err(e) => view! { <p class="text-red-400">{e.to_string()}</p> }.into_any(),
                        })}
                    </Suspense>
                </div>
            </div>
        </div>
    }
}

#[component]
fn StatCard(
    #[prop(into)] label: String,
    #[prop(into)] value: String,
    #[prop(into)] color: String,
) -> impl IntoView {
    let border_class = match color.as_str() {
        "blue" => "border-blue-500/30",
        "green" => "border-green-500/30",
        "purple" => "border-purple-500/30",
        "cyan" => "border-cyan-500/30",
        "red" => "border-red-500/30",
        "yellow" => "border-yellow-500/30",
        _ => "border-slate-700",
    };
    let value_class = match color.as_str() {
        "blue" => "text-blue-400",
        "green" => "text-green-400",
        "purple" => "text-purple-400",
        "cyan" => "text-cyan-400",
        "red" => "text-red-400",
        "yellow" => "text-yellow-400",
        _ => "text-white",
    };

    let card_class = format!("bg-slate-800 border {} rounded-xl p-5", border_class);
    let val_class = format!("text-3xl font-bold {}", value_class);

    view! {
        <div class=card_class>
            <p class="text-slate-400 text-sm mb-1">{label}</p>
            <p class=val_class>{value}</p>
        </div>
    }
}
