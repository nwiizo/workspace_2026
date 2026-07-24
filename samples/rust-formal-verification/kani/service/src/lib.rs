use axum::{
    Json, Router,
    http::StatusCode,
    routing::{get, post},
};
use kani_discount_verification::{DiscountRate, apply_discount};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct QuoteRequest {
    pub price: u16,
    pub discount_percent: u8,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct QuoteResponse {
    pub original_price: u16,
    pub discount_percent: u8,
    pub final_price: u16,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ErrorResponse {
    pub code: &'static str,
}

pub fn app() -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/quotes", post(create_quote))
}

async fn health() -> StatusCode {
    StatusCode::NO_CONTENT
}

async fn create_quote(
    Json(request): Json<QuoteRequest>,
) -> Result<Json<QuoteResponse>, (StatusCode, Json<ErrorResponse>)> {
    let Some(rate) = DiscountRate::new(request.discount_percent) else {
        return Err((
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(ErrorResponse {
                code: "invalid_discount_percent",
            }),
        ));
    };

    Ok(Json(QuoteResponse {
        original_price: request.price,
        discount_percent: rate.value(),
        final_price: apply_discount(request.price, rate),
    }))
}

#[cfg(test)]
mod tests {
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use serde_json::{Value, json};
    use tower::ServiceExt;

    use super::app;

    #[tokio::test]
    async fn returns_a_verified_quote() {
        let request = Request::post("/quotes")
            .header("content-type", "application/json")
            .body(Body::from(
                json!({ "price": 1_000, "discount_percent": 25 }).to_string(),
            ));
        let Ok(request) = request else {
            panic!("the test request must be valid");
        };

        let Ok(response) = app().oneshot(request).await;
        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), 1024).await;
        let Ok(body) = body else {
            panic!("the response body must be readable");
        };
        let decoded = serde_json::from_slice::<Value>(&body);
        let Ok(decoded) = decoded else {
            panic!("the response body must be JSON");
        };
        assert_eq!(
            decoded,
            json!({
                "original_price": 1_000,
                "discount_percent": 25,
                "final_price": 750
            })
        );
    }

    #[tokio::test]
    async fn rejects_an_invalid_discount_at_the_http_boundary() {
        let request = Request::post("/quotes")
            .header("content-type", "application/json")
            .body(Body::from(
                json!({ "price": 1_000, "discount_percent": 101 }).to_string(),
            ));
        let Ok(request) = request else {
            panic!("the test request must be valid");
        };

        let Ok(response) = app().oneshot(request).await;
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

        let body = to_bytes(response.into_body(), 1024).await;
        let Ok(body) = body else {
            panic!("the response body must be readable");
        };
        let decoded = serde_json::from_slice::<Value>(&body);
        let Ok(decoded) = decoded else {
            panic!("the response body must be JSON");
        };
        assert_eq!(decoded, json!({ "code": "invalid_discount_percent" }));
    }
}
