use topcoat::{
    Result,
    context::Cx,
    router::{Router, RouterBuilderDiscoverExt, page, path_param, query_params},
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

#[path_param(error = bad_request)]
struct ArticleId(u64);

#[page("/articles/{article_id}")]
async fn article(cx: &Cx) -> Result {
    let article_id = path_param::<ArticleId>(cx)?;
    view! { <h1>"Article " (article_id)</h1> }
}

#[query_params(error = bad_request)]
struct SearchQuery {
    q: Option<String>,
    page: Option<u32>,
}

#[page("/search")]
async fn search(cx: &Cx) -> Result {
    let query = query_params::<SearchQuery>(cx)?;
    let keyword = query.q.as_deref().unwrap_or("");
    let page_number = query.page.unwrap_or(1);
    view! {
        <form method="get" action="/search">
            <label for="q">"Keyword"</label>
            <input id="q" name="q">
            <button type="submit">"Search"</button>
        </form>
        <p>"keyword: " (keyword)</p>
        <p>"page: " (page_number)</p>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use topcoat::router::{Body, Request, StatusCode, to_bytes};

    async fn get(uri: &str) -> (StatusCode, String) {
        let response = router()
            .handle(
                Request::builder()
                    .uri(uri)
                    .body(Body::empty())
                    .expect("request should be valid"),
            )
            .await;
        let status_code = response.status();
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should be readable");
        (
            status_code,
            String::from_utf8(body.to_vec()).expect("body should be UTF-8"),
        )
    }

    #[tokio::test]
    async fn parses_typed_path_and_query_values() {
        let (status_code, html) = get("/articles/42").await;
        assert_eq!(status_code, StatusCode::OK);
        assert!(html.contains("Article 42"));

        let (status_code, html) = get("/search?q=rust&page=2").await;
        assert_eq!(status_code, StatusCode::OK);
        assert!(html.contains("keyword: rust"));
        assert!(html.contains("page: 2"));
    }

    #[tokio::test]
    async fn rejects_values_that_do_not_match_the_types() {
        assert_eq!(
            get("/articles/not-a-number").await.0,
            StatusCode::BAD_REQUEST
        );
        assert_eq!(get("/search?page=abc").await.0, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn escapes_query_values_in_html() {
        let (_, html) = get("/search?q=%3Cscript%3Ealert(1)%3C%2Fscript%3E").await;
        assert!(html.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
        assert!(!html.contains("<script>"));
    }
}
