//! SageMaker E2E tests.
mod common;
use aws_sdk_sagemaker::types::InstanceType;
use pretty_assertions::assert_eq;

#[tokio::test]
async fn e2e_sm_create_notebook_instance() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let sm = aws_sdk_sagemaker::Client::new(&cfg);
    let res = sm
        .create_notebook_instance()
        .notebook_instance_name("nb1")
        .instance_type(InstanceType::MlT2Medium)
        .role_arn("arn:aws:iam::000000000000:role/sm")
        .send()
        .await
        .unwrap();
    assert!(
        res.notebook_instance_arn()
            .unwrap()
            .contains(":notebook-instance/nb1")
    );
}

#[tokio::test]
async fn e2e_sm_describe_notebook() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let sm = aws_sdk_sagemaker::Client::new(&cfg);
    sm.create_notebook_instance()
        .notebook_instance_name("nb")
        .instance_type(InstanceType::MlT2Medium)
        .role_arn("arn:aws:iam::000000000000:role/sm")
        .send()
        .await
        .unwrap();
    let res = sm
        .describe_notebook_instance()
        .notebook_instance_name("nb")
        .send()
        .await
        .unwrap();
    assert_eq!(res.notebook_instance_name(), Some("nb"));
}

#[tokio::test]
async fn e2e_sm_create_model() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let sm = aws_sdk_sagemaker::Client::new(&cfg);
    let res = sm
        .create_model()
        .model_name("m1")
        .execution_role_arn("arn:aws:iam::000000000000:role/sm")
        .send()
        .await
        .unwrap();
    assert!(res.model_arn().unwrap().contains(":model/m1"));
}
