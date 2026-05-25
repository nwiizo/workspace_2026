//! Route 53 Resolver E2E tests.
mod common;
use aws_sdk_route53resolver::types::{IpAddressRequest, ResolverEndpointDirection, RuleTypeOption};
use pretty_assertions::assert_eq;

#[tokio::test]
async fn e2e_r53r_create_endpoint() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let r = aws_sdk_route53resolver::Client::new(&cfg);
    let res = r
        .create_resolver_endpoint()
        .creator_request_id("rid1")
        .direction(ResolverEndpointDirection::Inbound)
        .ip_addresses(
            IpAddressRequest::builder()
                .subnet_id("subnet-1")
                .build()
                .unwrap(),
        )
        .security_group_ids("sg-1")
        .send()
        .await
        .unwrap();
    let ep = res.resolver_endpoint().unwrap();
    assert!(ep.id().unwrap().starts_with("rslvr-"));
    assert_eq!(ep.direction(), Some(&ResolverEndpointDirection::Inbound));
}

#[tokio::test]
async fn e2e_r53r_list_endpoints() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let r = aws_sdk_route53resolver::Client::new(&cfg);
    r.create_resolver_endpoint()
        .creator_request_id("a")
        .direction(ResolverEndpointDirection::Outbound)
        .ip_addresses(
            IpAddressRequest::builder()
                .subnet_id("subnet-1")
                .build()
                .unwrap(),
        )
        .security_group_ids("sg-1")
        .send()
        .await
        .unwrap();
    let res = r.list_resolver_endpoints().send().await.unwrap();
    assert_eq!(res.resolver_endpoints().len(), 1);
}

#[tokio::test]
async fn e2e_r53r_create_resolver_rule() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let r = aws_sdk_route53resolver::Client::new(&cfg);
    let res = r
        .create_resolver_rule()
        .creator_request_id("rid2")
        .domain_name("example.com")
        .rule_type(RuleTypeOption::Forward)
        .send()
        .await
        .unwrap();
    assert_eq!(
        res.resolver_rule().unwrap().domain_name(),
        Some("example.com")
    );
}
