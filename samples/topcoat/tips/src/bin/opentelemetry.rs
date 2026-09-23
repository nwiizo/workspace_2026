use opentelemetry::trace::TracerProvider as _;
use opentelemetry_sdk::{Resource, trace::SdkTracerProvider};
use topcoat::{
    Result as TopcoatResult,
    context::CxBuilder,
    router::{Body, Next, Response, Router, RouterBuilderDiscoverExt, layer, method, page, uri},
    view::view,
};
use tracing::{Instrument, field, info_span};
use tracing_subscriber::{Registry, layer::SubscriberExt, util::SubscriberInitExt};

fn telemetry_resource() -> Resource {
    Resource::builder()
        .with_service_name("topcoat-tips")
        .build()
}

fn subscriber(provider: &SdkTracerProvider) -> impl tracing::Subscriber + Send + Sync + use<> {
    let tracer = provider.tracer("topcoat-tips");
    let telemetry = tracing_opentelemetry::layer()
        .with_tracer(tracer)
        .with_location(false)
        .with_tracked_inactivity(false)
        .with_threads(false)
        .with_target(false);
    Registry::default().with(telemetry)
}

fn init_tracing() -> SdkTracerProvider {
    let provider = SdkTracerProvider::builder()
        .with_simple_exporter(opentelemetry_stdout::SpanExporter::default())
        .with_resource(telemetry_resource())
        .build();
    subscriber(&provider).init();
    provider
}

fn router() -> Router {
    Router::builder().discover().build()
}

#[layer("/")]
async fn trace_request(cx: &mut CxBuilder, body: Body, next: Next<'_>) -> TopcoatResult<Response> {
    let span = info_span!(
        "http.request",
        "otel.kind" = "server",
        "http.request.method" = %method(cx),
        "url.path" = %uri(cx).path(),
        "http.response.status_code" = field::Empty,
    );
    let response = next.run(cx, body).instrument(span.clone()).await?;
    span.record(
        "http.response.status_code",
        i64::from(response.status().as_u16()),
    );
    Ok(response)
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let provider = init_tracing();
    let server_result = topcoat::start(router()).await;
    provider.shutdown()?;
    server_result?;
    Ok(())
}

#[page("/about")]
async fn about() -> TopcoatResult {
    view! { <h1>"About"</h1> }
}

#[cfg(test)]
mod tests {
    use super::*;
    use opentelemetry::{Key, Value, trace::SpanKind};
    use opentelemetry_sdk::trace::InMemorySpanExporter;
    use topcoat::router::{Body, Request, StatusCode};
    use tracing::instrument::WithSubscriber;

    #[tokio::test]
    async fn exports_the_approved_http_span_shape() {
        let exporter = InMemorySpanExporter::default();
        let provider = SdkTracerProvider::builder()
            .with_simple_exporter(exporter.clone())
            .with_resource(telemetry_resource())
            .build();

        let response = router()
            .handle(
                Request::builder()
                    .uri("/about")
                    .body(Body::empty())
                    .expect("request should be valid"),
            )
            .with_subscriber(subscriber(&provider))
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        provider.force_flush().expect("spans should flush");
        let spans = exporter
            .get_finished_spans()
            .expect("finished spans should be readable");
        assert_eq!(spans.len(), 1);
        let span = &spans[0];
        assert_eq!(span.name, "http.request");
        assert_eq!(span.span_kind, SpanKind::Server);
        assert_eq!(span.attributes.len(), 3);

        let attribute = |key: &str| {
            span.attributes
                .iter()
                .find(|attribute| attribute.key.as_str() == key)
                .map(|attribute| &attribute.value)
        };
        assert_eq!(
            attribute("http.request.method"),
            Some(&Value::String("GET".into()))
        );
        assert_eq!(attribute("url.path"), Some(&Value::String("/about".into())));
        assert_eq!(
            attribute("http.response.status_code"),
            Some(&Value::I64(200))
        );
        assert_eq!(
            telemetry_resource().get(&Key::new("service.name")),
            Some(Value::String("topcoat-tips".into()))
        );
    }
}
