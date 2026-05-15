//! SNS E2E tests against AWS official API spec, including SNS→SQS fanout.
//!
//! References:
//! - CreateTopic:        <https://docs.aws.amazon.com/sns/latest/api/API_CreateTopic.html>
//! - Publish:            <https://docs.aws.amazon.com/sns/latest/api/API_Publish.html>
//! - Subscribe:          <https://docs.aws.amazon.com/sns/latest/api/API_Subscribe.html>
//! - SNS→SQS delivery:   <https://docs.aws.amazon.com/sns/latest/dg/sns-sqs-as-subscriber.html>

mod common;

use pretty_assertions::assert_eq;

#[tokio::test]
async fn e2e_sns_create_topic_returns_arn() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let sns = aws_sdk_sns::Client::new(&cfg);
    let res = sns.create_topic().name("t1").send().await.unwrap();
    let arn = res.topic_arn().unwrap();
    assert!(arn.starts_with("arn:aws:sns:"));
    assert!(arn.ends_with(":t1"));
}

#[tokio::test]
async fn e2e_sns_list_topics_includes_created() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let sns = aws_sdk_sns::Client::new(&cfg);
    for n in ["a", "b"] {
        sns.create_topic().name(n).send().await.unwrap();
    }
    let res = sns.list_topics().send().await.unwrap();
    let arns: Vec<&str> = res.topics().iter().filter_map(|t| t.topic_arn()).collect();
    assert_eq!(arns.len(), 2);
}

#[tokio::test]
async fn e2e_sns_publish_returns_message_id() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let sns = aws_sdk_sns::Client::new(&cfg);
    let topic = sns
        .create_topic()
        .name("pub")
        .send()
        .await
        .unwrap()
        .topic_arn()
        .unwrap()
        .to_string();
    let res = sns
        .publish()
        .topic_arn(topic)
        .message("hello")
        .send()
        .await
        .unwrap();
    assert!(res.message_id().is_some());
}

#[tokio::test]
async fn e2e_sns_subscribe_then_publish_fans_out_to_sqs_queue() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let sns = aws_sdk_sns::Client::new(&cfg);
    let sqs = aws_sdk_sqs::Client::new(&cfg);

    // Create SQS queue (the destination).
    let q = sqs.create_queue().queue_name("destq").send().await.unwrap();
    let queue_url = q.queue_url().unwrap().to_string();
    let queue_arn = "arn:aws:sqs:us-east-1:000000000000:destq";

    // Create SNS topic and subscribe the queue.
    let topic_arn = sns
        .create_topic()
        .name("fanout")
        .send()
        .await
        .unwrap()
        .topic_arn()
        .unwrap()
        .to_string();
    sns.subscribe()
        .topic_arn(&topic_arn)
        .protocol("sqs")
        .endpoint(queue_arn)
        .send()
        .await
        .unwrap();

    sns.publish()
        .topic_arn(&topic_arn)
        .message("from-sns")
        .subject("hi")
        .send()
        .await
        .unwrap();

    let r = sqs
        .receive_message()
        .queue_url(queue_url)
        .send()
        .await
        .unwrap();
    let msg = r.messages().first().expect("queue must receive a message");
    let body: serde_json::Value = serde_json::from_str(msg.body().unwrap()).unwrap();
    assert_eq!(body["Type"], "Notification");
    assert_eq!(body["Message"], "from-sns");
    assert_eq!(body["Subject"], "hi");
}

#[tokio::test]
async fn e2e_sns_list_subscriptions_by_topic_filters_correctly() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let sns = aws_sdk_sns::Client::new(&cfg);

    let topic_a = sns
        .create_topic()
        .name("a")
        .send()
        .await
        .unwrap()
        .topic_arn()
        .unwrap()
        .to_string();
    let topic_b = sns
        .create_topic()
        .name("b")
        .send()
        .await
        .unwrap()
        .topic_arn()
        .unwrap()
        .to_string();
    sns.subscribe()
        .topic_arn(&topic_a)
        .protocol("email")
        .endpoint("alpha@example.com")
        .send()
        .await
        .unwrap();
    sns.subscribe()
        .topic_arn(&topic_b)
        .protocol("email")
        .endpoint("beta@example.com")
        .send()
        .await
        .unwrap();

    let res = sns
        .list_subscriptions_by_topic()
        .topic_arn(&topic_a)
        .send()
        .await
        .unwrap();
    assert_eq!(res.subscriptions().len(), 1);
    assert_eq!(res.subscriptions()[0].endpoint(), Some("alpha@example.com"));
}

#[tokio::test]
async fn e2e_sns_delete_topic_removes_subscriptions() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let sns = aws_sdk_sns::Client::new(&cfg);

    let topic_arn = sns
        .create_topic()
        .name("doomed")
        .send()
        .await
        .unwrap()
        .topic_arn()
        .unwrap()
        .to_string();
    sns.subscribe()
        .topic_arn(&topic_arn)
        .protocol("email")
        .endpoint("x@example.com")
        .send()
        .await
        .unwrap();
    sns.delete_topic()
        .topic_arn(&topic_arn)
        .send()
        .await
        .unwrap();
    let res = sns.list_subscriptions().send().await.unwrap();
    assert!(
        res.subscriptions()
            .iter()
            .all(|s| s.topic_arn() != Some(topic_arn.as_str()))
    );
}
