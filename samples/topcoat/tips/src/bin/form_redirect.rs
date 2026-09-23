use serde::Deserialize;
use topcoat::{
    Result,
    router::{
        Router, RouterBuilderDiscoverExt,
        content::Form,
        error::{SeeOther, bad_request, see_other},
        route,
    },
};

fn router() -> Router {
    Router::builder().discover().build()
}

#[tokio::main]
async fn main() -> topcoat::Result<()> {
    topcoat::start(router()).await?;
    Ok(())
}

#[derive(Deserialize)]
struct ContactForm {
    name: String,
}

#[route(POST "/contact")]
async fn create_contact(Form(input): Form<ContactForm>) -> Result<SeeOther> {
    if input.name.trim().is_empty() {
        return Err(bad_request("name is required").into());
    }

    Ok(see_other("/thanks"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use topcoat::router::{Body, Request, StatusCode, header};

    async fn post(body: &'static str) -> topcoat::router::Response {
        router()
            .handle(
                Request::builder()
                    .method("POST")
                    .uri("/contact")
                    .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                    .body(Body::from(body))
                    .expect("request should be valid"),
            )
            .await
    }

    #[tokio::test]
    async fn redirects_after_accepting_a_form() {
        let response = post("name=Alice").await;
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        assert_eq!(
            response.headers().get(header::LOCATION),
            Some(&header::HeaderValue::from_static("/thanks"))
        );
    }

    #[tokio::test]
    async fn rejects_an_empty_name() {
        assert_eq!(post("name=%20").await.status(), StatusCode::BAD_REQUEST);
    }
}
