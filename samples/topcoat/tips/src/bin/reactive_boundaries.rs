use topcoat::{
    Result,
    router::{Router, RouterBuilderDiscoverExt, error::bad_request, page},
    runtime::{Event, procedure, shard},
    view::{component, view},
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
    view! {
        <!DOCTYPE html>
        <html lang="ja">
            <head>
                <meta charset="utf-8">
                <title>"Reactive boundaries"</title>
                topcoat::dev::script()
            </head>
            <body>
                local_toggle()
                save_button()
                search_box()
            </body>
        </html>
    }
}

#[component]
async fn local_toggle() -> Result {
    view! {
        signal opened = false;
        <section>
            <h2>"signal"</h2>
            <button @click=$(|_event: Event| opened.toggle())>
                $(if opened.get() { "Hide" } else { "Show" })
            </button>
            <p :hidden=$(!opened.get())>"This changes in the browser."</p>
        </section>
    }
}

#[component]
async fn save_button() -> Result {
    view! {
        signal status = String::new();
        <section>
            <h2>"procedure"</h2>
            <button @click=$(async |_event: Event| {
                let result = save_message("hello".to_owned()).await;
                status.set(result);
            })>"Save"</button>
            <p aria-live="polite">$(status.get())</p>
        </section>
    }
}

#[procedure]
async fn save_message(message: String) -> Result<String> {
    let message = message.trim();
    if message.is_empty() || message.chars().count() > 80 {
        return Err(bad_request("message must be between 1 and 80 characters").into());
    }
    Ok(format!("saved: {message}"))
}

#[component]
async fn search_box() -> Result {
    view! {
        signal query = String::new();
        <section>
            <h2>"shard"</h2>
            <input
                type="search"
                aria-label="Search"
                maxlength="80"
                @input=$(|event: Event| query.set(event.target.value))
            >
            search_results(query: $(query.get()))
        </section>
    }
}

#[shard]
async fn search_results(query: String) -> Result {
    if query.chars().count() > 80 {
        return Err(bad_request("query must be at most 80 characters").into());
    }
    let query = query.trim();
    view! {
        if query.is_empty() {
            <p>"Enter a search term."</p>
        } else {
            <p>"server result: " (query)</p>
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use topcoat::router::{Body, Request, StatusCode, to_bytes};

    #[tokio::test]
    async fn renders_all_three_boundaries() {
        let response = router()
            .handle(
                Request::builder()
                    .uri("/")
                    .body(Body::empty())
                    .expect("request should be valid"),
            )
            .await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should be readable");
        let html = String::from_utf8(body.to_vec()).expect("body should be UTF-8");
        for heading in ["signal", "procedure", "shard"] {
            assert!(html.contains(heading));
        }
    }
}
