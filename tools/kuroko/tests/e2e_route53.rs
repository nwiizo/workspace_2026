//! Route 53 E2E tests against AWS official API spec.
//!
//! References:
//! - CreateHostedZone:         <https://docs.aws.amazon.com/Route53/latest/APIReference/API_CreateHostedZone.html>
//! - ChangeResourceRecordSets: <https://docs.aws.amazon.com/Route53/latest/APIReference/API_ChangeResourceRecordSets.html>
//! - ListResourceRecordSets:   <https://docs.aws.amazon.com/Route53/latest/APIReference/API_ListResourceRecordSets.html>

mod common;

use aws_sdk_route53::types::{
    Change, ChangeAction, ChangeBatch, ResourceRecord, ResourceRecordSet, RrType,
};
use pretty_assertions::assert_eq;

#[tokio::test]
async fn e2e_r53_create_hosted_zone_returns_id() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let r53 = aws_sdk_route53::Client::new(&cfg);

    let res = r53
        .create_hosted_zone()
        .name("example.com")
        .caller_reference("e2e-1")
        .send()
        .await
        .unwrap();
    let zone = res.hosted_zone().unwrap();
    assert!(zone.id().starts_with("/hostedzone/Z"));
    assert_eq!(zone.name(), "example.com.");
}

#[tokio::test]
async fn e2e_r53_list_zones_after_create() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let r53 = aws_sdk_route53::Client::new(&cfg);

    r53.create_hosted_zone()
        .name("alpha.test")
        .caller_reference("1")
        .send()
        .await
        .unwrap();
    r53.create_hosted_zone()
        .name("beta.test")
        .caller_reference("2")
        .send()
        .await
        .unwrap();
    let list = r53.list_hosted_zones().send().await.unwrap();
    assert_eq!(list.hosted_zones().len(), 2);
}

fn change(action: ChangeAction, name: &str, rtype: RrType, ttl: i64, value: &str) -> Change {
    Change::builder()
        .action(action)
        .resource_record_set(
            ResourceRecordSet::builder()
                .name(name)
                .r#type(rtype)
                .ttl(ttl)
                .resource_records(ResourceRecord::builder().value(value).build().unwrap())
                .build()
                .unwrap(),
        )
        .build()
        .unwrap()
}

#[tokio::test]
async fn e2e_r53_create_record_set_then_list() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let r53 = aws_sdk_route53::Client::new(&cfg);

    let res = r53
        .create_hosted_zone()
        .name("zone.test")
        .caller_reference("e2e")
        .send()
        .await
        .unwrap();
    let zone_id = res.hosted_zone().unwrap().id().to_string();
    let id = zone_id.trim_start_matches("/hostedzone/");

    r53.change_resource_record_sets()
        .hosted_zone_id(id)
        .change_batch(
            ChangeBatch::builder()
                .changes(change(
                    ChangeAction::Create,
                    "www.zone.test.",
                    RrType::A,
                    300,
                    "203.0.113.10",
                ))
                .build()
                .unwrap(),
        )
        .send()
        .await
        .unwrap();

    let list = r53
        .list_resource_record_sets()
        .hosted_zone_id(id)
        .send()
        .await
        .unwrap();
    let rrs = list.resource_record_sets();
    assert_eq!(rrs.len(), 1);
    assert_eq!(rrs[0].name(), "www.zone.test.");
    assert_eq!(rrs[0].r#type(), &RrType::A);
    assert_eq!(rrs[0].resource_records().len(), 1);
    assert_eq!(rrs[0].resource_records()[0].value(), "203.0.113.10");
}

#[tokio::test]
async fn e2e_r53_upsert_replaces_existing_record() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let r53 = aws_sdk_route53::Client::new(&cfg);

    let res = r53
        .create_hosted_zone()
        .name("zone.test")
        .caller_reference("e2e")
        .send()
        .await
        .unwrap();
    let id = res
        .hosted_zone()
        .unwrap()
        .id()
        .trim_start_matches("/hostedzone/")
        .to_string();

    r53.change_resource_record_sets()
        .hosted_zone_id(&id)
        .change_batch(
            ChangeBatch::builder()
                .changes(change(
                    ChangeAction::Create,
                    "x.zone.test.",
                    RrType::A,
                    300,
                    "1.1.1.1",
                ))
                .build()
                .unwrap(),
        )
        .send()
        .await
        .unwrap();
    r53.change_resource_record_sets()
        .hosted_zone_id(&id)
        .change_batch(
            ChangeBatch::builder()
                .changes(change(
                    ChangeAction::Upsert,
                    "x.zone.test.",
                    RrType::A,
                    60,
                    "2.2.2.2",
                ))
                .build()
                .unwrap(),
        )
        .send()
        .await
        .unwrap();
    let list = r53
        .list_resource_record_sets()
        .hosted_zone_id(&id)
        .send()
        .await
        .unwrap();
    assert_eq!(list.resource_record_sets().len(), 1);
    assert_eq!(list.resource_record_sets()[0].ttl(), Some(60));
    assert_eq!(
        list.resource_record_sets()[0].resource_records()[0].value(),
        "2.2.2.2"
    );
}

#[tokio::test]
async fn e2e_r53_delete_record_set() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let r53 = aws_sdk_route53::Client::new(&cfg);

    let res = r53
        .create_hosted_zone()
        .name("zone.test")
        .caller_reference("e2e")
        .send()
        .await
        .unwrap();
    let id = res
        .hosted_zone()
        .unwrap()
        .id()
        .trim_start_matches("/hostedzone/")
        .to_string();
    r53.change_resource_record_sets()
        .hosted_zone_id(&id)
        .change_batch(
            ChangeBatch::builder()
                .changes(change(
                    ChangeAction::Create,
                    "y.zone.test.",
                    RrType::A,
                    300,
                    "1.1.1.1",
                ))
                .build()
                .unwrap(),
        )
        .send()
        .await
        .unwrap();
    r53.change_resource_record_sets()
        .hosted_zone_id(&id)
        .change_batch(
            ChangeBatch::builder()
                .changes(change(
                    ChangeAction::Delete,
                    "y.zone.test.",
                    RrType::A,
                    300,
                    "1.1.1.1",
                ))
                .build()
                .unwrap(),
        )
        .send()
        .await
        .unwrap();
    let list = r53
        .list_resource_record_sets()
        .hosted_zone_id(&id)
        .send()
        .await
        .unwrap();
    assert_eq!(list.resource_record_sets().len(), 0);
}

#[tokio::test]
async fn e2e_r53_delete_hosted_zone() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let r53 = aws_sdk_route53::Client::new(&cfg);

    let res = r53
        .create_hosted_zone()
        .name("doomed.test")
        .caller_reference("e2e")
        .send()
        .await
        .unwrap();
    let id = res
        .hosted_zone()
        .unwrap()
        .id()
        .trim_start_matches("/hostedzone/")
        .to_string();
    r53.delete_hosted_zone().id(&id).send().await.unwrap();
    let err = r53.get_hosted_zone().id(&id).send().await;
    assert!(err.is_err(), "zone must be gone after delete");
}
