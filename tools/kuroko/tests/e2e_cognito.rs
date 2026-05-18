//! Cognito Identity Provider E2E tests against AWS official API spec.
//!
//! References:
//! - CreateUserPool:        <https://docs.aws.amazon.com/cognito-user-identity-pools/latest/APIReference/API_CreateUserPool.html>
//! - CreateUserPoolClient:  <https://docs.aws.amazon.com/cognito-user-identity-pools/latest/APIReference/API_CreateUserPoolClient.html>
//! - AdminCreateUser:       <https://docs.aws.amazon.com/cognito-user-identity-pools/latest/APIReference/API_AdminCreateUser.html>
//! - AdminGetUser:          <https://docs.aws.amazon.com/cognito-user-identity-pools/latest/APIReference/API_AdminGetUser.html>

mod common;

use aws_sdk_cognitoidentityprovider::types::AttributeType;
use pretty_assertions::assert_eq;

#[tokio::test]
async fn e2e_cognito_create_user_pool_returns_id_and_arn() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let c = aws_sdk_cognitoidentityprovider::Client::new(&cfg);

    let res = c
        .create_user_pool()
        .pool_name("kuroko-pool")
        .send()
        .await
        .unwrap();
    let pool = res.user_pool().unwrap();
    assert_eq!(pool.name(), Some("kuroko-pool"));
    assert!(pool.id().unwrap().starts_with("us-east-1_"));
    assert!(pool.arn().unwrap().starts_with("arn:aws:cognito-idp:"));
}

#[tokio::test]
async fn e2e_cognito_list_user_pools() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let c = aws_sdk_cognitoidentityprovider::Client::new(&cfg);

    for n in ["alpha", "beta"] {
        c.create_user_pool().pool_name(n).send().await.unwrap();
    }
    let res = c.list_user_pools().max_results(60).send().await.unwrap();
    assert_eq!(res.user_pools().len(), 2);
}

#[tokio::test]
async fn e2e_cognito_create_user_pool_client_with_secret() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let c = aws_sdk_cognitoidentityprovider::Client::new(&cfg);

    let pool = c
        .create_user_pool()
        .pool_name("p")
        .send()
        .await
        .unwrap()
        .user_pool()
        .unwrap()
        .id()
        .unwrap()
        .to_string();
    let res = c
        .create_user_pool_client()
        .user_pool_id(&pool)
        .client_name("web")
        .generate_secret(true)
        .send()
        .await
        .unwrap();
    let client = res.user_pool_client().unwrap();
    assert_eq!(client.client_name(), Some("web"));
    assert!(client.client_secret().is_some());
}

#[tokio::test]
async fn e2e_cognito_admin_create_user_then_get() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let c = aws_sdk_cognitoidentityprovider::Client::new(&cfg);

    let pool = c
        .create_user_pool()
        .pool_name("p")
        .send()
        .await
        .unwrap()
        .user_pool()
        .unwrap()
        .id()
        .unwrap()
        .to_string();
    c.admin_create_user()
        .user_pool_id(&pool)
        .username("alice@example.com")
        .user_attributes(
            AttributeType::builder()
                .name("email")
                .value("alice@example.com")
                .build()
                .unwrap(),
        )
        .send()
        .await
        .unwrap();
    let got = c
        .admin_get_user()
        .user_pool_id(&pool)
        .username("alice@example.com")
        .send()
        .await
        .unwrap();
    assert_eq!(got.username(), "alice@example.com");
    let attrs = got.user_attributes();
    assert!(
        attrs
            .iter()
            .any(|a| a.name() == "email" && a.value() == Some("alice@example.com"))
    );
}

#[tokio::test]
async fn e2e_cognito_admin_create_duplicate_user_fails() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let c = aws_sdk_cognitoidentityprovider::Client::new(&cfg);

    let pool = c
        .create_user_pool()
        .pool_name("p")
        .send()
        .await
        .unwrap()
        .user_pool()
        .unwrap()
        .id()
        .unwrap()
        .to_string();
    c.admin_create_user()
        .user_pool_id(&pool)
        .username("u")
        .send()
        .await
        .unwrap();
    let err = c
        .admin_create_user()
        .user_pool_id(&pool)
        .username("u")
        .send()
        .await;
    assert!(err.is_err(), "duplicate username must fail");
}

#[tokio::test]
async fn e2e_cognito_admin_delete_user() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let c = aws_sdk_cognitoidentityprovider::Client::new(&cfg);

    let pool = c
        .create_user_pool()
        .pool_name("p")
        .send()
        .await
        .unwrap()
        .user_pool()
        .unwrap()
        .id()
        .unwrap()
        .to_string();
    c.admin_create_user()
        .user_pool_id(&pool)
        .username("doomed")
        .send()
        .await
        .unwrap();
    c.admin_delete_user()
        .user_pool_id(&pool)
        .username("doomed")
        .send()
        .await
        .unwrap();
    let err = c
        .admin_get_user()
        .user_pool_id(&pool)
        .username("doomed")
        .send()
        .await;
    assert!(err.is_err(), "user must be gone");
}
