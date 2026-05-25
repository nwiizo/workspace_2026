//! GameLift E2E.
mod common;
use pretty_assertions::assert_eq;

#[tokio::test]
async fn e2e_gl_create_build() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let c = aws_sdk_gamelift::Client::new(&cfg);
    c.create_build()
        .name("game-build")
        .version("1.0.0")
        .send()
        .await
        .unwrap();
    let listed = c.list_builds().send().await.unwrap();
    assert_eq!(listed.builds().len(), 1);
}

#[tokio::test]
async fn e2e_gl_create_fleet_and_list() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let c = aws_sdk_gamelift::Client::new(&cfg);
    let res = c
        .create_fleet()
        .name("fleet1")
        .ec2_instance_type(aws_sdk_gamelift::types::Ec2InstanceType::C5Large)
        .build_id("build-x")
        .compute_type(aws_sdk_gamelift::types::ComputeType::Ec2)
        .send()
        .await
        .unwrap();
    assert!(res.fleet_attributes().is_some());
    let listed = c.list_fleets().send().await.unwrap();
    assert_eq!(listed.fleet_ids().len(), 1);
}
