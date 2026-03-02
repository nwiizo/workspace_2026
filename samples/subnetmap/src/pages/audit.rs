use leptos::prelude::*;

use crate::server::audit::list_audit_logs;

#[component]
pub fn AuditPage() -> impl IntoView {
    let logs = Resource::new(|| (), |_| list_audit_logs(None, None, None, Some(100)));

    view! {
        <div>
            <h1 class="text-2xl font-bold text-white mb-6">"Audit Log"</h1>

            <div class="bg-slate-800 border border-slate-700 rounded-xl">
                <Suspense fallback=|| view! { <div class="p-5 text-slate-400">"Loading..."</div> }>
                    {move || logs.get().map(|result| match result {
                        Ok(list) => {
                            if list.is_empty() {
                                view! {
                                    <div class="text-center py-12 text-slate-500">
                                        <p>"No audit log entries"</p>
                                    </div>
                                }.into_any()
                            } else {
                                view! {
                                    <table class="w-full text-sm">
                                        <thead>
                                            <tr class="border-b border-slate-700 text-slate-400 text-left">
                                                <th class="p-4 font-medium">"Time"</th>
                                                <th class="p-4 font-medium">"Action"</th>
                                                <th class="p-4 font-medium">"Entity Type"</th>
                                                <th class="p-4 font-medium">"Entity ID"</th>
                                            </tr>
                                        </thead>
                                        <tbody>
                                            {list.into_iter().map(|log| {
                                                let time = log.created_at.format("%Y-%m-%d %H:%M:%S").to_string();
                                                let action = log.action.clone();
                                                let action_class = match log.action.as_str() {
                                                    "create" => "text-green-400",
                                                    "update" => "text-yellow-400",
                                                    "delete" => "text-red-400",
                                                    _ => "text-slate-300",
                                                };
                                                let entity_type = log.entity_type.clone();
                                                let entity_id = log.entity_id.to_string().chars().take(8).collect::<String>();
                                                view! {
                                                    <tr class="border-b border-slate-800 hover:bg-slate-800/50">
                                                        <td class="p-4 text-slate-500 text-xs font-mono">{time}</td>
                                                        <td class="p-4">
                                                            <span class=action_class>{action}</span>
                                                        </td>
                                                        <td class="p-4 text-slate-300">{entity_type}</td>
                                                        <td class="p-4 text-slate-500 font-mono text-xs">{entity_id}</td>
                                                    </tr>
                                                }
                                            }).collect::<Vec<_>>()}
                                        </tbody>
                                    </table>
                                }.into_any()
                            }
                        },
                        Err(e) => view! { <p class="p-5 text-red-400">{e.to_string()}</p> }.into_any(),
                    })}
                </Suspense>
            </div>
        </div>
    }
}
