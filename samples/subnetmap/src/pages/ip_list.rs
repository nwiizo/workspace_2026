use leptos::prelude::*;

use crate::components::ip_table::IpTable;
use crate::server::ip_addresses::list_ip_addresses;

#[component]
pub fn IpListPage() -> impl IntoView {
    let (status_filter, set_status_filter) = signal(String::new());

    let ips = Resource::new(
        move || status_filter.get(),
        |filter| async move {
            let f = if filter.is_empty() {
                None
            } else {
                Some(filter)
            };
            list_ip_addresses(None, f, None, Some(100)).await
        },
    );

    view! {
        <div>
            <div class="flex justify-between items-center mb-6">
                <h1 class="text-2xl font-bold text-white">"IP Addresses"</h1>
            </div>

            <div class="flex gap-2 mb-6">
                <FilterButton label="All" value="" current=status_filter set=set_status_filter />
                <FilterButton label="Available" value="available" current=status_filter set=set_status_filter />
                <FilterButton label="Assigned" value="assigned" current=status_filter set=set_status_filter />
                <FilterButton label="Reserved" value="reserved" current=status_filter set=set_status_filter />
                <FilterButton label="Deprecated" value="deprecated" current=status_filter set=set_status_filter />
            </div>

            <div class="bg-slate-800 border border-slate-700 rounded-xl p-5">
                <Suspense fallback=|| view! { <div class="text-slate-400">"Loading..."</div> }>
                    {move || ips.get().map(|result| match result {
                        Ok(list) => {
                            if list.is_empty() {
                                view! {
                                    <p class="text-slate-500 text-sm text-center py-8">"No IP addresses found"</p>
                                }.into_any()
                            } else {
                                view! { <IpTable ips=list /> }.into_any()
                            }
                        },
                        Err(e) => view! { <p class="text-red-400">{e.to_string()}</p> }.into_any(),
                    })}
                </Suspense>
            </div>
        </div>
    }
}

#[component]
fn FilterButton(
    label: &'static str,
    value: &'static str,
    current: ReadSignal<String>,
    set: WriteSignal<String>,
) -> impl IntoView {
    let is_active = move || current.get() == value;

    view! {
        <button
            on:click=move |_| set.set(value.to_string())
            class=move || if is_active() {
                "px-3 py-1.5 rounded-lg text-sm font-medium bg-blue-500 text-white"
            } else {
                "px-3 py-1.5 rounded-lg text-sm font-medium bg-slate-700 text-slate-300 hover:bg-slate-600"
            }>
            {label}
        </button>
    }
}
