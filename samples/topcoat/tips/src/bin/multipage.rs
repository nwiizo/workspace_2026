use topcoat::{
    Result,
    router::{Router, RouterBuilderDiscoverExt, layout, page},
    view::view,
};

fn router() -> Router {
    Router::builder().discover().build()
}

#[tokio::main]
async fn main() -> topcoat::Result<()> {
    topcoat::start(router()).await?;
    Ok(())
}

#[page("/")]
async fn home() -> Result {
    view! { <main><h1>"Home"</h1></main> }
}

#[layout("/")]
async fn root_layout(slot: Result) -> Result {
    view! {
        <!DOCTYPE html>
        <html lang="ja">
            <head><meta charset="utf-8"><title>"Topcoat Pages"</title></head>
            <body>
                <nav>
                    <a href="/">"Home"</a>
                    " "
                    <a href="/about">"About"</a>
                    " "
                    <a href="/status">"Status"</a>
                </nav>
                (slot?)
            </body>
        </html>
    }
}

#[page("/about")]
async fn about() -> Result {
    view! { <main><h1>"About"</h1></main> }
}

#[page("/status")]
async fn status_page() -> Result {
    view! { <main><h1>"Status"</h1></main> }
}

#[cfg(test)]
mod tests {
    use super::*;
    use topcoat::router::{Body, Request, StatusCode, to_bytes};

    async fn get(path: &str) -> (StatusCode, String) {
        let response = router()
            .handle(
                Request::builder()
                    .uri(path)
                    .body(Body::empty())
                    .expect("request should be valid"),
            )
            .await;
        let status = response.status();
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should be readable");
        let html = String::from_utf8(body.to_vec()).expect("body should be UTF-8");
        (status, html)
    }

    #[tokio::test]
    async fn three_pages_share_navigation() {
        for (path, heading) in [
            ("/", "<h1>Home</h1>"),
            ("/about", "<h1>About</h1>"),
            ("/status", "<h1>Status</h1>"),
        ] {
            let (status, html) = get(path).await;
            assert_eq!(status, StatusCode::OK);
            assert!(html.contains("<nav>"));
            assert!(html.contains(heading));
        }
    }
}
