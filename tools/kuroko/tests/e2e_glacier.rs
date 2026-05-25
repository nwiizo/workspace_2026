//! Glacier E2E tests.
mod common;
use pretty_assertions::assert_eq;

#[tokio::test]
async fn e2e_glacier_create_vault() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let g = aws_sdk_glacier::Client::new(&cfg);
    let res = g
        .create_vault()
        .account_id("-")
        .vault_name("backup")
        .send()
        .await
        .unwrap();
    assert!(res.location().unwrap().contains("backup"));
}

#[tokio::test]
async fn e2e_glacier_describe_vault() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let g = aws_sdk_glacier::Client::new(&cfg);
    g.create_vault()
        .account_id("-")
        .vault_name("v")
        .send()
        .await
        .unwrap();
    let res = g
        .describe_vault()
        .account_id("-")
        .vault_name("v")
        .send()
        .await
        .unwrap();
    assert_eq!(res.vault_name(), Some("v"));
}

#[tokio::test]
async fn e2e_glacier_list_vaults() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let g = aws_sdk_glacier::Client::new(&cfg);
    for n in ["a", "b"] {
        g.create_vault()
            .account_id("-")
            .vault_name(n)
            .send()
            .await
            .unwrap();
    }
    let res = g.list_vaults().account_id("-").send().await.unwrap();
    assert_eq!(res.vault_list().len(), 2);
}
