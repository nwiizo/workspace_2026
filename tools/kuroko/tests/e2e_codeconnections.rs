//! CodeConnections E2E.
mod common;
use aws_sdk_codeconnections::types::ProviderType;
use pretty_assertions::assert_eq;

#[tokio::test]
async fn e2e_cn_create_get_connection() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let c = aws_sdk_codeconnections::Client::new(&cfg);
    let res = c
        .create_connection()
        .connection_name("github-conn")
        .provider_type(ProviderType::Github)
        .send()
        .await
        .unwrap();
    let arn = res.connection_arn().to_string();
    let got = c
        .get_connection()
        .connection_arn(&arn)
        .send()
        .await
        .unwrap();
    assert_eq!(
        got.connection().unwrap().connection_name(),
        Some("github-conn")
    );
}

#[tokio::test]
async fn e2e_cn_list_connections() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let c = aws_sdk_codeconnections::Client::new(&cfg);
    for n in ["a", "b"] {
        c.create_connection()
            .connection_name(n)
            .provider_type(ProviderType::Github)
            .send()
            .await
            .unwrap();
    }
    let res = c.list_connections().send().await.unwrap();
    assert_eq!(res.connections().len(), 2);
}
