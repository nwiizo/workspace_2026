//! EC2 E2E tests against AWS official API spec.
//!
//! References:
//! - DescribeRegions:           <https://docs.aws.amazon.com/AWSEC2/latest/APIReference/API_DescribeRegions.html>
//! - CreateVpc:                 <https://docs.aws.amazon.com/AWSEC2/latest/APIReference/API_CreateVpc.html>
//! - CreateSubnet:              <https://docs.aws.amazon.com/AWSEC2/latest/APIReference/API_CreateSubnet.html>
//! - CreateSecurityGroup:       <https://docs.aws.amazon.com/AWSEC2/latest/APIReference/API_CreateSecurityGroup.html>
//! - AuthorizeSecurityGroupIngress: <https://docs.aws.amazon.com/AWSEC2/latest/APIReference/API_AuthorizeSecurityGroupIngress.html>
//! - RunInstances:              <https://docs.aws.amazon.com/AWSEC2/latest/APIReference/API_RunInstances.html>

mod common;

use aws_sdk_ec2::types::{InstanceStateName, InstanceType};
use pretty_assertions::assert_eq;

#[tokio::test]
async fn e2e_ec2_describe_regions_returns_known_list() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let ec2 = aws_sdk_ec2::Client::new(&cfg);

    let res = ec2.describe_regions().send().await.unwrap();
    let names: Vec<&str> = res
        .regions()
        .iter()
        .filter_map(|r| r.region_name())
        .collect();
    assert!(names.contains(&"us-east-1"));
    assert!(names.contains(&"ap-northeast-1"));
}

#[tokio::test]
async fn e2e_ec2_describe_availability_zones() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let ec2 = aws_sdk_ec2::Client::new(&cfg);

    let res = ec2.describe_availability_zones().send().await.unwrap();
    let zones: Vec<&str> = res
        .availability_zones()
        .iter()
        .filter_map(|z| z.zone_name())
        .collect();
    assert!(zones.contains(&"us-east-1a"));
}

#[tokio::test]
async fn e2e_ec2_create_vpc_then_describe() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let ec2 = aws_sdk_ec2::Client::new(&cfg);

    let res = ec2
        .create_vpc()
        .cidr_block("10.0.0.0/16")
        .send()
        .await
        .unwrap();
    let vpc_id = res.vpc().unwrap().vpc_id().unwrap().to_string();
    assert!(vpc_id.starts_with("vpc-"));

    let described = ec2.describe_vpcs().vpc_ids(&vpc_id).send().await.unwrap();
    assert_eq!(described.vpcs().len(), 1);
    assert_eq!(described.vpcs()[0].cidr_block(), Some("10.0.0.0/16"));
}

#[tokio::test]
async fn e2e_ec2_create_subnet_under_vpc() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let ec2 = aws_sdk_ec2::Client::new(&cfg);

    let vpc_id = ec2
        .create_vpc()
        .cidr_block("10.1.0.0/16")
        .send()
        .await
        .unwrap()
        .vpc()
        .unwrap()
        .vpc_id()
        .unwrap()
        .to_string();
    let subnet_id = ec2
        .create_subnet()
        .vpc_id(&vpc_id)
        .cidr_block("10.1.1.0/24")
        .availability_zone("us-east-1a")
        .send()
        .await
        .unwrap()
        .subnet()
        .unwrap()
        .subnet_id()
        .unwrap()
        .to_string();
    assert!(subnet_id.starts_with("subnet-"));

    let described = ec2
        .describe_subnets()
        .subnet_ids(&subnet_id)
        .send()
        .await
        .unwrap();
    assert_eq!(described.subnets()[0].vpc_id(), Some(vpc_id.as_str()));
}

#[tokio::test]
async fn e2e_ec2_create_subnet_under_missing_vpc_fails() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let ec2 = aws_sdk_ec2::Client::new(&cfg);

    let err = ec2
        .create_subnet()
        .vpc_id("vpc-nope")
        .cidr_block("10.0.0.0/24")
        .send()
        .await;
    assert!(
        err.is_err(),
        "missing VPC must fail with InvalidVpcID.NotFound"
    );
}

#[tokio::test]
async fn e2e_ec2_security_group_with_ingress() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let ec2 = aws_sdk_ec2::Client::new(&cfg);

    let sg_id = ec2
        .create_security_group()
        .group_name("web")
        .description("allow 443")
        .send()
        .await
        .unwrap()
        .group_id()
        .unwrap()
        .to_string();
    assert!(sg_id.starts_with("sg-"));

    use aws_sdk_ec2::types::{IpPermission, IpRange};
    ec2.authorize_security_group_ingress()
        .group_id(&sg_id)
        .ip_permissions(
            IpPermission::builder()
                .ip_protocol("tcp")
                .from_port(443)
                .to_port(443)
                .ip_ranges(IpRange::builder().cidr_ip("0.0.0.0/0").build())
                .build(),
        )
        .send()
        .await
        .unwrap();

    let described = ec2
        .describe_security_groups()
        .group_ids(&sg_id)
        .send()
        .await
        .unwrap();
    assert_eq!(described.security_groups()[0].ip_permissions().len(), 1);
}

#[tokio::test]
async fn e2e_ec2_run_then_terminate_instance() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let ec2 = aws_sdk_ec2::Client::new(&cfg);

    let res = ec2
        .run_instances()
        .image_id("ami-test")
        .instance_type(InstanceType::T3Micro)
        .min_count(1)
        .max_count(1)
        .send()
        .await
        .unwrap();
    let inst_id = res.instances()[0].instance_id().unwrap().to_string();
    assert!(inst_id.starts_with("i-"));
    assert_eq!(
        res.instances()[0].state().and_then(|s| s.name()),
        Some(&InstanceStateName::Running)
    );

    ec2.terminate_instances()
        .instance_ids(&inst_id)
        .send()
        .await
        .unwrap();
    let described = ec2
        .describe_instances()
        .instance_ids(&inst_id)
        .send()
        .await
        .unwrap();
    let inst = &described.reservations()[0].instances()[0];
    assert_eq!(
        inst.state().and_then(|s| s.name()),
        Some(&InstanceStateName::Terminated)
    );
}

#[tokio::test]
async fn e2e_ec2_run_multiple_instances_via_max_count() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let ec2 = aws_sdk_ec2::Client::new(&cfg);

    let res = ec2
        .run_instances()
        .image_id("ami-test")
        .instance_type(InstanceType::T3Micro)
        .min_count(3)
        .max_count(3)
        .send()
        .await
        .unwrap();
    assert_eq!(res.instances().len(), 3);
}
