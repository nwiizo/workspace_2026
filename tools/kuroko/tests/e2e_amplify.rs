//! AWS Amplify E2E.
mod common;
use pretty_assertions::assert_eq;

#[tokio::test]
async fn e2e_amplify_create_get_app() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let c = aws_sdk_amplify::Client::new(&cfg);
    let res = c
        .create_app()
        .name("my-app")
        .description("test")
        .send()
        .await
        .unwrap();
    let id = res.app().unwrap().app_id().to_string();
    let got = c.get_app().app_id(&id).send().await.unwrap();
    assert_eq!(got.app().unwrap().name(), "my-app");
}

#[tokio::test]
async fn e2e_amplify_list_apps() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let c = aws_sdk_amplify::Client::new(&cfg);
    for n in ["a", "b"] {
        c.create_app().name(n).send().await.unwrap();
    }
    let res = c.list_apps().send().await.unwrap();
    assert_eq!(res.apps().len(), 2);
}
