//! EventBridge E2E tests against AWS official API spec.
//!
//! References:
//! - CreateEventBus: <https://docs.aws.amazon.com/eventbridge/latest/APIReference/API_CreateEventBus.html>
//! - PutRule:        <https://docs.aws.amazon.com/eventbridge/latest/APIReference/API_PutRule.html>
//! - PutTargets:     <https://docs.aws.amazon.com/eventbridge/latest/APIReference/API_PutTargets.html>
//! - PutEvents:      <https://docs.aws.amazon.com/eventbridge/latest/APIReference/API_PutEvents.html>

mod common;

use aws_sdk_eventbridge::types::{PutEventsRequestEntry, RuleState, Target};
use pretty_assertions::assert_eq;

#[tokio::test]
async fn e2e_eb_default_bus_exists() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let eb = aws_sdk_eventbridge::Client::new(&cfg);

    let res = eb.list_event_buses().send().await.unwrap();
    let names: Vec<&str> = res.event_buses().iter().filter_map(|b| b.name()).collect();
    assert!(
        names.contains(&"default"),
        "default bus must exist: {names:?}"
    );
}

#[tokio::test]
async fn e2e_eb_create_then_delete_custom_bus() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let eb = aws_sdk_eventbridge::Client::new(&cfg);

    let res = eb.create_event_bus().name("custom").send().await.unwrap();
    let arn = res.event_bus_arn().unwrap();
    assert!(arn.starts_with("arn:aws:events:"));

    eb.delete_event_bus().name("custom").send().await.unwrap();
}

#[tokio::test]
async fn e2e_eb_put_rule_with_pattern() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let eb = aws_sdk_eventbridge::Client::new(&cfg);

    let res = eb
        .put_rule()
        .name("orders")
        .event_pattern(r#"{"source": ["shop.orders"]}"#)
        .state(RuleState::Enabled)
        .send()
        .await
        .unwrap();
    assert!(res.rule_arn().unwrap().contains("rule/orders"));

    let desc = eb.describe_rule().name("orders").send().await.unwrap();
    assert_eq!(desc.state(), Some(&RuleState::Enabled));
}

#[tokio::test]
async fn e2e_eb_put_events_with_matching_rule_fans_out_to_sqs() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let eb = aws_sdk_eventbridge::Client::new(&cfg);
    let sqs = aws_sdk_sqs::Client::new(&cfg);

    // Set up an SQS destination queue.
    let q = sqs
        .create_queue()
        .queue_name("orders-q")
        .send()
        .await
        .unwrap();
    let queue_url = q.queue_url().unwrap().to_string();
    let queue_arn = "arn:aws:sqs:us-east-1:000000000000:orders-q";

    // Rule that matches `shop.orders` events.
    eb.put_rule()
        .name("orders-rule")
        .event_pattern(r#"{"source": ["shop.orders"]}"#)
        .send()
        .await
        .unwrap();
    eb.put_targets()
        .rule("orders-rule")
        .targets(
            Target::builder()
                .id("queue-target")
                .arn(queue_arn)
                .build()
                .unwrap(),
        )
        .send()
        .await
        .unwrap();

    // Send an event that matches the pattern.
    let entry = PutEventsRequestEntry::builder()
        .source("shop.orders")
        .detail_type("OrderCreated")
        .detail(r#"{"order_id": 1, "total": 42.5}"#)
        .build();
    let res = eb.put_events().entries(entry).send().await.unwrap();
    assert_eq!(res.failed_entry_count(), 0);

    // The queue should now have one EventBridge envelope.
    let recv = sqs
        .receive_message()
        .queue_url(queue_url)
        .send()
        .await
        .unwrap();
    assert_eq!(recv.messages().len(), 1);
    let body: serde_json::Value = serde_json::from_str(recv.messages()[0].body().unwrap()).unwrap();
    assert_eq!(body["source"], "shop.orders");
    assert_eq!(body["detail-type"], "OrderCreated");
}

#[tokio::test]
async fn e2e_eb_non_matching_event_is_dropped() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let eb = aws_sdk_eventbridge::Client::new(&cfg);
    let sqs = aws_sdk_sqs::Client::new(&cfg);

    let q = sqs
        .create_queue()
        .queue_name("nope-q")
        .send()
        .await
        .unwrap();
    let queue_url = q.queue_url().unwrap().to_string();
    let queue_arn = "arn:aws:sqs:us-east-1:000000000000:nope-q";

    eb.put_rule()
        .name("strict")
        .event_pattern(r#"{"source": ["only.this"]}"#)
        .send()
        .await
        .unwrap();
    eb.put_targets()
        .rule("strict")
        .targets(Target::builder().id("t").arn(queue_arn).build().unwrap())
        .send()
        .await
        .unwrap();

    let entry = PutEventsRequestEntry::builder()
        .source("other.source")
        .detail_type("X")
        .detail("{}")
        .build();
    eb.put_events().entries(entry).send().await.unwrap();

    let recv = sqs
        .receive_message()
        .queue_url(queue_url)
        .send()
        .await
        .unwrap();
    assert_eq!(
        recv.messages().len(),
        0,
        "non-matching event must not be delivered"
    );
}

#[tokio::test]
async fn e2e_eb_disabled_rule_does_not_fire() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let eb = aws_sdk_eventbridge::Client::new(&cfg);
    let sqs = aws_sdk_sqs::Client::new(&cfg);

    let q = sqs.create_queue().queue_name("dis-q").send().await.unwrap();
    let queue_url = q.queue_url().unwrap().to_string();
    let queue_arn = "arn:aws:sqs:us-east-1:000000000000:dis-q";

    eb.put_rule()
        .name("off")
        .event_pattern(r#"{"source": ["x"]}"#)
        .send()
        .await
        .unwrap();
    eb.put_targets()
        .rule("off")
        .targets(Target::builder().id("t").arn(queue_arn).build().unwrap())
        .send()
        .await
        .unwrap();
    eb.disable_rule().name("off").send().await.unwrap();

    let entry = PutEventsRequestEntry::builder()
        .source("x")
        .detail_type("X")
        .detail("{}")
        .build();
    eb.put_events().entries(entry).send().await.unwrap();
    let recv = sqs
        .receive_message()
        .queue_url(queue_url)
        .send()
        .await
        .unwrap();
    assert_eq!(recv.messages().len(), 0);
}

#[tokio::test]
async fn e2e_eb_remove_target() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let eb = aws_sdk_eventbridge::Client::new(&cfg);

    eb.put_rule()
        .name("r")
        .event_pattern(r#"{"source": ["x"]}"#)
        .send()
        .await
        .unwrap();
    eb.put_targets()
        .rule("r")
        .targets(
            Target::builder()
                .id("t1")
                .arn("arn:aws:sqs:us-east-1:000000000000:q")
                .build()
                .unwrap(),
        )
        .send()
        .await
        .unwrap();
    eb.remove_targets()
        .rule("r")
        .ids("t1")
        .send()
        .await
        .unwrap();
    let ts = eb.list_targets_by_rule().rule("r").send().await.unwrap();
    assert_eq!(ts.targets().len(), 0);
}
