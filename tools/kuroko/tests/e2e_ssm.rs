//! SSM Parameter Store E2E tests against AWS official API spec.
//!
//! References:
//! - PutParameter:        <https://docs.aws.amazon.com/systems-manager/latest/APIReference/API_PutParameter.html>
//! - GetParameter:        <https://docs.aws.amazon.com/systems-manager/latest/APIReference/API_GetParameter.html>
//! - GetParameters:       <https://docs.aws.amazon.com/systems-manager/latest/APIReference/API_GetParameters.html>
//! - GetParametersByPath: <https://docs.aws.amazon.com/systems-manager/latest/APIReference/API_GetParametersByPath.html>

mod common;

use aws_sdk_ssm::types::ParameterType;
use pretty_assertions::assert_eq;

#[tokio::test]
async fn e2e_ssm_put_then_get_string() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let ssm = aws_sdk_ssm::Client::new(&cfg);

    ssm.put_parameter()
        .name("/app/db/host")
        .value("db.internal")
        .r#type(ParameterType::String)
        .send()
        .await
        .unwrap();

    let got = ssm
        .get_parameter()
        .name("/app/db/host")
        .send()
        .await
        .unwrap();
    let p = got.parameter().unwrap();
    assert_eq!(p.value(), Some("db.internal"));
    assert_eq!(p.r#type(), Some(&ParameterType::String));
}

#[tokio::test]
async fn e2e_ssm_duplicate_without_overwrite_fails() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let ssm = aws_sdk_ssm::Client::new(&cfg);

    ssm.put_parameter()
        .name("/k")
        .value("v1")
        .send()
        .await
        .unwrap();
    let err = ssm.put_parameter().name("/k").value("v2").send().await;
    assert!(
        err.is_err(),
        "second PutParameter without Overwrite must fail"
    );
}

#[tokio::test]
async fn e2e_ssm_overwrite_increments_version() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let ssm = aws_sdk_ssm::Client::new(&cfg);

    ssm.put_parameter()
        .name("/k")
        .value("v1")
        .send()
        .await
        .unwrap();
    let res = ssm
        .put_parameter()
        .name("/k")
        .value("v2")
        .overwrite(true)
        .send()
        .await
        .unwrap();
    assert_eq!(res.version(), 2);
}

#[tokio::test]
async fn e2e_ssm_get_parameters_separates_found_and_invalid() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let ssm = aws_sdk_ssm::Client::new(&cfg);

    ssm.put_parameter()
        .name("/exists")
        .value("v")
        .send()
        .await
        .unwrap();
    let res = ssm
        .get_parameters()
        .names("/exists")
        .names("/missing")
        .send()
        .await
        .unwrap();
    assert_eq!(res.parameters().len(), 1);
    assert_eq!(res.invalid_parameters().len(), 1);
    assert_eq!(res.invalid_parameters()[0], "/missing");
}

#[tokio::test]
async fn e2e_ssm_get_parameters_by_path_filters_immediate_level() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let ssm = aws_sdk_ssm::Client::new(&cfg);

    for (n, v) in [
        ("/app/host", "h"),
        ("/app/port", "p"),
        ("/app/nested/x", "x"),
        ("/other/y", "y"),
    ] {
        ssm.put_parameter().name(n).value(v).send().await.unwrap();
    }

    let shallow = ssm
        .get_parameters_by_path()
        .path("/app")
        .send()
        .await
        .unwrap();
    assert_eq!(shallow.parameters().len(), 2);

    let recursive = ssm
        .get_parameters_by_path()
        .path("/app")
        .recursive(true)
        .send()
        .await
        .unwrap();
    assert_eq!(recursive.parameters().len(), 3);
}

#[tokio::test]
async fn e2e_ssm_delete_parameter_removes_it() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let ssm = aws_sdk_ssm::Client::new(&cfg);

    ssm.put_parameter()
        .name("/gone")
        .value("v")
        .send()
        .await
        .unwrap();
    ssm.delete_parameter().name("/gone").send().await.unwrap();
    let err = ssm.get_parameter().name("/gone").send().await;
    assert!(err.is_err(), "GetParameter must fail after DeleteParameter");
}
