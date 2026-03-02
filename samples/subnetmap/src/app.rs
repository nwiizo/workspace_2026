use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::{
    components::{Route, Router, Routes},
    path,
};

use crate::components::layout::Layout;
use crate::pages;

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="ja" class="dark">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <AutoReload options=options.clone() />
                <HydrationScripts options />
                <MetaTags />
            </head>
            <body class="bg-slate-950 text-slate-100 min-h-screen">
                <App />
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    view! {
        <Stylesheet id="leptos" href="/pkg/subnetmap.css" />
        <Title text="SubnetMap" />

        <Router>
            <Layout>
                <Routes fallback=|| "Page not found.".into_view()>
                    <Route path=path!("/") view=pages::dashboard::DashboardPage />
                    <Route path=path!("/subnets") view=pages::subnet_list::SubnetListPage />
                    <Route path=path!("/subnets/:id") view=pages::subnet_detail::SubnetDetailPage />
                    <Route path=path!("/ips") view=pages::ip_list::IpListPage />
                    <Route path=path!("/ips/:id") view=pages::ip_detail::IpDetailPage />
                    <Route path=path!("/vlans") view=pages::vlan_list::VlanListPage />
                    <Route path=path!("/vlans/:id") view=pages::vlan_detail::VlanDetailPage />
                    <Route path=path!("/audit") view=pages::audit::AuditPage />
                    <Route path=path!("/search") view=pages::search::SearchPage />
                    <Route path=path!("/settings") view=pages::settings::SettingsPage />
                </Routes>
            </Layout>
        </Router>
    }
}
