//! Elastic Beanstalk E2E.
//! Refs: <https://docs.aws.amazon.com/elasticbeanstalk/latest/api/Welcome.html>

mod common;
use pretty_assertions::assert_eq;

#[tokio::test]
async fn e2e_eb_create_describe_app() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let c = aws_sdk_elasticbeanstalk::Client::new(&cfg);
    c.create_application()
        .application_name("my-app")
        .description("test")
        .send()
        .await
        .unwrap();
    let res = c.describe_applications().send().await.unwrap();
    assert_eq!(res.applications().len(), 1);
    assert_eq!(res.applications()[0].application_name(), Some("my-app"));
}

#[tokio::test]
async fn e2e_eb_duplicate_app_fails() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let c = aws_sdk_elasticbeanstalk::Client::new(&cfg);
    c.create_application()
        .application_name("dup")
        .send()
        .await
        .unwrap();
    let err = c.create_application().application_name("dup").send().await;
    assert!(err.is_err());
}

#[tokio::test]
async fn e2e_eb_delete_app() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let c = aws_sdk_elasticbeanstalk::Client::new(&cfg);
    c.create_application()
        .application_name("a")
        .send()
        .await
        .unwrap();
    c.delete_application()
        .application_name("a")
        .send()
        .await
        .unwrap();
    let res = c.describe_applications().send().await.unwrap();
    assert_eq!(res.applications().len(), 0);
}

#[tokio::test]
async fn e2e_eb_create_environment() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let c = aws_sdk_elasticbeanstalk::Client::new(&cfg);
    c.create_application()
        .application_name("svc")
        .send()
        .await
        .unwrap();
    let res = c
        .create_environment()
        .application_name("svc")
        .environment_name("svc-prod")
        .solution_stack_name("64bit Amazon Linux 2 v3.5.7 running Docker")
        .send()
        .await
        .unwrap();
    assert_eq!(res.environment_name(), Some("svc-prod"));
    assert_eq!(res.application_name(), Some("svc"));
}
