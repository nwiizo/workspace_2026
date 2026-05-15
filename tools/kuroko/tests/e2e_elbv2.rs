//! ELBv2 E2E tests against AWS official API spec.
//!
//! References:
//! - CreateLoadBalancer:  <https://docs.aws.amazon.com/elasticloadbalancing/latest/APIReference/API_CreateLoadBalancer.html>
//! - CreateTargetGroup:   <https://docs.aws.amazon.com/elasticloadbalancing/latest/APIReference/API_CreateTargetGroup.html>
//! - RegisterTargets:     <https://docs.aws.amazon.com/elasticloadbalancing/latest/APIReference/API_RegisterTargets.html>
//! - DescribeTargetHealth:<https://docs.aws.amazon.com/elasticloadbalancing/latest/APIReference/API_DescribeTargetHealth.html>

mod common;

use aws_sdk_elasticloadbalancingv2::types::{
    Action, ActionTypeEnum, ProtocolEnum, TargetDescription,
};
use pretty_assertions::assert_eq;

#[tokio::test]
async fn e2e_elbv2_create_load_balancer_returns_dns() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let elb = aws_sdk_elasticloadbalancingv2::Client::new(&cfg);

    let res = elb
        .create_load_balancer()
        .name("alb-web")
        .send()
        .await
        .unwrap();
    let lb = &res.load_balancers()[0];
    assert_eq!(lb.load_balancer_name(), Some("alb-web"));
    assert!(lb.dns_name().unwrap().contains("elb.amazonaws.com"));
}

#[tokio::test]
async fn e2e_elbv2_duplicate_load_balancer_fails() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let elb = aws_sdk_elasticloadbalancingv2::Client::new(&cfg);

    elb.create_load_balancer().name("dup").send().await.unwrap();
    let err = elb.create_load_balancer().name("dup").send().await;
    assert!(err.is_err(), "duplicate must fail");
}

#[tokio::test]
async fn e2e_elbv2_create_target_group_and_register() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let elb = aws_sdk_elasticloadbalancingv2::Client::new(&cfg);

    let tg = elb
        .create_target_group()
        .name("tg")
        .protocol(ProtocolEnum::Http)
        .port(80)
        .send()
        .await
        .unwrap();
    let tg_arn = tg.target_groups()[0]
        .target_group_arn()
        .unwrap()
        .to_string();

    elb.register_targets()
        .target_group_arn(&tg_arn)
        .targets(TargetDescription::builder().id("i-1234").port(80).build())
        .send()
        .await
        .unwrap();
    let health = elb
        .describe_target_health()
        .target_group_arn(&tg_arn)
        .send()
        .await
        .unwrap();
    assert_eq!(health.target_health_descriptions().len(), 1);
    let h = &health.target_health_descriptions()[0];
    assert_eq!(h.target().unwrap().id(), Some("i-1234"));
}

#[tokio::test]
async fn e2e_elbv2_deregister_targets() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let elb = aws_sdk_elasticloadbalancingv2::Client::new(&cfg);

    let tg = elb
        .create_target_group()
        .name("tg2")
        .protocol(ProtocolEnum::Http)
        .port(80)
        .send()
        .await
        .unwrap();
    let arn = tg.target_groups()[0]
        .target_group_arn()
        .unwrap()
        .to_string();

    elb.register_targets()
        .target_group_arn(&arn)
        .targets(TargetDescription::builder().id("i-1").port(80).build())
        .targets(TargetDescription::builder().id("i-2").port(80).build())
        .send()
        .await
        .unwrap();
    elb.deregister_targets()
        .target_group_arn(&arn)
        .targets(TargetDescription::builder().id("i-1").port(80).build())
        .send()
        .await
        .unwrap();
    let health = elb
        .describe_target_health()
        .target_group_arn(&arn)
        .send()
        .await
        .unwrap();
    assert_eq!(health.target_health_descriptions().len(), 1);
}

#[tokio::test]
async fn e2e_elbv2_create_listener_with_default_action() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let elb = aws_sdk_elasticloadbalancingv2::Client::new(&cfg);

    let lb = elb
        .create_load_balancer()
        .name("lb-listen")
        .send()
        .await
        .unwrap();
    let lb_arn = lb.load_balancers()[0]
        .load_balancer_arn()
        .unwrap()
        .to_string();

    let tg = elb
        .create_target_group()
        .name("tg-listen")
        .protocol(ProtocolEnum::Http)
        .port(80)
        .send()
        .await
        .unwrap();
    let tg_arn = tg.target_groups()[0]
        .target_group_arn()
        .unwrap()
        .to_string();

    let res = elb
        .create_listener()
        .load_balancer_arn(&lb_arn)
        .protocol(ProtocolEnum::Http)
        .port(80)
        .default_actions(
            Action::builder()
                .r#type(ActionTypeEnum::Forward)
                .target_group_arn(&tg_arn)
                .build(),
        )
        .send()
        .await
        .unwrap();
    let listener = &res.listeners()[0];
    assert_eq!(listener.port(), Some(80));

    let listeners = elb
        .describe_listeners()
        .load_balancer_arn(&lb_arn)
        .send()
        .await
        .unwrap();
    assert_eq!(listeners.listeners().len(), 1);
}

#[tokio::test]
async fn e2e_elbv2_delete_lb_cascades_listeners() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let elb = aws_sdk_elasticloadbalancingv2::Client::new(&cfg);

    let lb = elb.create_load_balancer().name("dlb").send().await.unwrap();
    let lb_arn = lb.load_balancers()[0]
        .load_balancer_arn()
        .unwrap()
        .to_string();

    elb.create_listener()
        .load_balancer_arn(&lb_arn)
        .protocol(ProtocolEnum::Http)
        .port(80)
        .send()
        .await
        .unwrap();
    elb.delete_load_balancer()
        .load_balancer_arn(&lb_arn)
        .send()
        .await
        .unwrap();
    let listeners = elb
        .describe_listeners()
        .load_balancer_arn(lb_arn)
        .send()
        .await
        .unwrap();
    assert_eq!(listeners.listeners().len(), 0);
}
