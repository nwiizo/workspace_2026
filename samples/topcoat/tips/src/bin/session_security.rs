use topcoat::{
    Result,
    cookie::RouterBuilderCookieExt,
    router::{
        Router, RouterBuilderDiscoverExt,
        error::{SeeOther, see_other},
        route,
    },
    session::{RouterBuilderSessionExt, SessionConfig},
};

fn router() -> Router {
    Router::builder()
        .cookies()
        .sessions(SessionConfig::default())
        .discover()
        .build()
}

#[tokio::main]
async fn main() -> topcoat::Result<()> {
    topcoat::start(router()).await?;
    Ok(())
}

#[route(POST "/settings")]
async fn update_settings() -> Result<SeeOther> {
    Ok(see_other("/settings/saved"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use topcoat::router::{Body, Request, StatusCode};

    async fn post(sec_fetch_site: Option<&str>) -> topcoat::router::Response {
        let mut request = Request::builder().method("POST").uri("/settings");
        if let Some(value) = sec_fetch_site {
            request = request.header("sec-fetch-site", value);
        }
        router()
            .handle(
                request
                    .body(Body::empty())
                    .expect("request should be valid"),
            )
            .await
    }

    #[tokio::test]
    async fn allows_same_origin_and_rejects_cross_site_posts() {
        assert_eq!(post(None).await.status(), StatusCode::SEE_OTHER);
        assert_eq!(
            post(Some("cross-site")).await.status(),
            StatusCode::FORBIDDEN
        );
    }
}
