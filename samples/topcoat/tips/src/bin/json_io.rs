use serde::{Deserialize, Serialize};
use topcoat::{
    Result,
    router::{Router, RouterBuilderDiscoverExt, content::Json, route},
};

fn router() -> Router {
    Router::builder().discover().build()
}

#[tokio::main]
async fn main() -> topcoat::Result<()> {
    topcoat::start(router()).await?;
    Ok(())
}

#[derive(Deserialize, Serialize)]
struct Message {
    text: String,
}

#[route(POST "/api/echo")]
async fn echo(Json(input): Json<Message>) -> Result<Json<Message>> {
    Ok(Json(input))
}

#[cfg(test)]
mod tests {
    use super::*;
    use topcoat::router::{Body, Request, StatusCode, header, to_bytes};

    async fn post(body: &'static str) -> (StatusCode, String, Option<String>) {
        let response = router()
            .handle(
                Request::builder()
                    .method("POST")
                    .uri("/api/echo")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(body))
                    .expect("request should be valid"),
            )
            .await;
        let status_code = response.status();
        let content_type = response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should be readable");
        (
            status_code,
            String::from_utf8(bytes.to_vec()).expect("body should be UTF-8"),
            content_type,
        )
    }

    #[tokio::test]
    async fn returns_the_typed_json_body() {
        let (status_code, body, content_type) = post(r#"{"text":"hello"}"#).await;
        assert_eq!(status_code, StatusCode::OK);
        assert_eq!(body, r#"{"text":"hello"}"#);
        assert_eq!(content_type.as_deref(), Some("application/json"));
    }

    #[tokio::test]
    async fn rejects_json_without_the_required_field() {
        assert_eq!(post(r#"{"wrong":true}"#).await.0, StatusCode::BAD_REQUEST);
    }
}
