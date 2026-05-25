//! AWS Batch E2E tests.
//! Refs: <https://docs.aws.amazon.com/batch/latest/APIReference/Welcome.html>

mod common;

use aws_sdk_batch::types::{JobDefinitionType, JqState};
use pretty_assertions::assert_eq;

#[tokio::test]
async fn e2e_batch_register_job_definition() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let b = aws_sdk_batch::Client::new(&cfg);
    let res = b
        .register_job_definition()
        .job_definition_name("etl")
        .r#type(JobDefinitionType::Container)
        .send()
        .await
        .unwrap();
    assert_eq!(res.job_definition_name(), Some("etl"));
    assert_eq!(res.revision(), Some(1));
}

#[tokio::test]
async fn e2e_batch_register_increments_revision() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let b = aws_sdk_batch::Client::new(&cfg);
    b.register_job_definition()
        .job_definition_name("d")
        .r#type(JobDefinitionType::Container)
        .send()
        .await
        .unwrap();
    let r2 = b
        .register_job_definition()
        .job_definition_name("d")
        .r#type(JobDefinitionType::Container)
        .send()
        .await
        .unwrap();
    assert_eq!(r2.revision(), Some(2));
}

#[tokio::test]
async fn e2e_batch_create_job_queue() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let b = aws_sdk_batch::Client::new(&cfg);
    let res = b
        .create_job_queue()
        .job_queue_name("q1")
        .state(JqState::Enabled)
        .priority(10)
        .send()
        .await
        .unwrap();
    assert_eq!(res.job_queue_name(), Some("q1"));
}

#[tokio::test]
async fn e2e_batch_submit_job() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let b = aws_sdk_batch::Client::new(&cfg);
    b.create_job_queue()
        .job_queue_name("q")
        .priority(1)
        .send()
        .await
        .unwrap();
    b.register_job_definition()
        .job_definition_name("jd")
        .r#type(JobDefinitionType::Container)
        .send()
        .await
        .unwrap();
    let res = b
        .submit_job()
        .job_name("test-job")
        .job_queue("q")
        .job_definition("jd")
        .send()
        .await
        .unwrap();
    assert!(!res.job_id().unwrap().is_empty());
}

#[tokio::test]
async fn e2e_batch_describe_jobs_returns_submitted() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let b = aws_sdk_batch::Client::new(&cfg);
    b.create_job_queue()
        .job_queue_name("q")
        .send()
        .await
        .unwrap();
    b.register_job_definition()
        .job_definition_name("jd")
        .r#type(JobDefinitionType::Container)
        .send()
        .await
        .unwrap();
    let id = b
        .submit_job()
        .job_name("j")
        .job_queue("q")
        .job_definition("jd")
        .send()
        .await
        .unwrap()
        .job_id
        .unwrap();
    let res = b.describe_jobs().jobs(id).send().await.unwrap();
    assert_eq!(res.jobs().len(), 1);
}
