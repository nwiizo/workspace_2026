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
            <body class="bg-black text-white min-h-screen">
                <App />
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    view! {
        <Stylesheet id="leptos" href="/pkg/chirp.css" />
        <Title text="Chirp" />

        <Router>
            <Layout>
                <Routes fallback=|| "Page not found.".into_view()>
                    <Route path=path!("/") view=pages::home::HomePage />
                    <Route path=path!("/login") view=pages::login::LoginPage />
                    <Route path=path!("/signup") view=pages::signup::SignupPage />
                    <Route path=path!("/post/:id") view=pages::post_detail::PostDetailPage />
                    <Route
                        path=path!("/user/:username")
                        view=pages::profile::ProfilePage
                    />
                    <Route
                        path=path!("/notifications")
                        view=pages::notifications::NotificationsPage
                    />
                    <Route path=path!("/explore") view=pages::explore::ExplorePage />
                    <Route path=path!("/search") view=pages::search::SearchPage />
                    <Route path=path!("/settings") view=pages::settings::SettingsPage />
                </Routes>
            </Layout>
        </Router>
    }
}
