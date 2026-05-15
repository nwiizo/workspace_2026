//! API Gateway (v1) E2E tests against AWS official API spec.
//!
//! References:
//! - CreateRestApi:   <https://docs.aws.amazon.com/apigateway/latest/api/API_CreateRestApi.html>
//! - CreateResource:  <https://docs.aws.amazon.com/apigateway/latest/api/API_CreateResource.html>
//! - PutMethod:       <https://docs.aws.amazon.com/apigateway/latest/api/API_PutMethod.html>
//! - CreateDeployment:<https://docs.aws.amazon.com/apigateway/latest/api/API_CreateDeployment.html>

mod common;

use pretty_assertions::assert_eq;

#[tokio::test]
async fn e2e_apigw_create_rest_api_returns_id_and_root_resource() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let agw = aws_sdk_apigateway::Client::new(&cfg);

    let res = agw.create_rest_api().name("my-api").send().await.unwrap();
    let id = res.id().unwrap().to_string();
    assert_eq!(res.name(), Some("my-api"));

    let resources = agw.get_resources().rest_api_id(&id).send().await.unwrap();
    let items = resources.items();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].path(), Some("/"));
}

#[tokio::test]
async fn e2e_apigw_create_resource_under_root() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let agw = aws_sdk_apigateway::Client::new(&cfg);

    let api_id = agw
        .create_rest_api()
        .name("api")
        .send()
        .await
        .unwrap()
        .id()
        .unwrap()
        .to_string();
    let root_id = agw
        .get_resources()
        .rest_api_id(&api_id)
        .send()
        .await
        .unwrap()
        .items()[0]
        .id()
        .unwrap()
        .to_string();
    let res = agw
        .create_resource()
        .rest_api_id(&api_id)
        .parent_id(&root_id)
        .path_part("orders")
        .send()
        .await
        .unwrap();
    assert_eq!(res.path(), Some("/orders"));
    assert_eq!(res.parent_id(), Some(root_id.as_str()));
}

#[tokio::test]
async fn e2e_apigw_put_method_and_integration() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let agw = aws_sdk_apigateway::Client::new(&cfg);

    let api_id = agw
        .create_rest_api()
        .name("api")
        .send()
        .await
        .unwrap()
        .id()
        .unwrap()
        .to_string();
    let root_id = agw
        .get_resources()
        .rest_api_id(&api_id)
        .send()
        .await
        .unwrap()
        .items()[0]
        .id()
        .unwrap()
        .to_string();
    let resource_id = agw
        .create_resource()
        .rest_api_id(&api_id)
        .parent_id(&root_id)
        .path_part("things")
        .send()
        .await
        .unwrap()
        .id()
        .unwrap()
        .to_string();

    agw.put_method()
        .rest_api_id(&api_id)
        .resource_id(&resource_id)
        .http_method("GET")
        .authorization_type("NONE")
        .send()
        .await
        .unwrap();

    let m = agw
        .get_method()
        .rest_api_id(&api_id)
        .resource_id(&resource_id)
        .http_method("GET")
        .send()
        .await
        .unwrap();
    assert_eq!(m.http_method(), Some("GET"));

    agw.put_integration()
        .rest_api_id(&api_id)
        .resource_id(&resource_id)
        .http_method("GET")
        .r#type(aws_sdk_apigateway::types::IntegrationType::AwsProxy)
        .integration_http_method("POST")
        .uri("arn:aws:apigateway:us-east-1:lambda:path/2015-03-31/functions/x/invocations")
        .send()
        .await
        .unwrap();

    let i = agw
        .get_integration()
        .rest_api_id(&api_id)
        .resource_id(&resource_id)
        .http_method("GET")
        .send()
        .await
        .unwrap();
    assert_eq!(
        i.r#type(),
        Some(&aws_sdk_apigateway::types::IntegrationType::AwsProxy)
    );
}

#[tokio::test]
async fn e2e_apigw_create_deployment_then_stage() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let agw = aws_sdk_apigateway::Client::new(&cfg);

    let api_id = agw
        .create_rest_api()
        .name("api")
        .send()
        .await
        .unwrap()
        .id()
        .unwrap()
        .to_string();
    let deployment = agw
        .create_deployment()
        .rest_api_id(&api_id)
        .send()
        .await
        .unwrap();
    let dep_id = deployment.id().unwrap().to_string();

    agw.create_stage()
        .rest_api_id(&api_id)
        .stage_name("prod")
        .deployment_id(&dep_id)
        .send()
        .await
        .unwrap();
    let stage = agw
        .get_stage()
        .rest_api_id(&api_id)
        .stage_name("prod")
        .send()
        .await
        .unwrap();
    assert_eq!(stage.deployment_id(), Some(dep_id.as_str()));
}

#[tokio::test]
async fn e2e_apigw_get_rest_apis_lists_created() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let agw = aws_sdk_apigateway::Client::new(&cfg);

    for n in ["a", "b"] {
        agw.create_rest_api().name(n).send().await.unwrap();
    }
    let list = agw.get_rest_apis().send().await.unwrap();
    assert_eq!(list.items().len(), 2);
}

#[tokio::test]
async fn e2e_apigw_delete_rest_api() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let agw = aws_sdk_apigateway::Client::new(&cfg);

    let id = agw
        .create_rest_api()
        .name("doomed")
        .send()
        .await
        .unwrap()
        .id()
        .unwrap()
        .to_string();
    agw.delete_rest_api().rest_api_id(&id).send().await.unwrap();
    let err = agw.get_rest_api().rest_api_id(&id).send().await;
    assert!(err.is_err(), "rest api must be gone");
}
