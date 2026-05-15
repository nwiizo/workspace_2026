//! SQS E2E tests verifying behavior matches the AWS official API spec.
//!
//! References:
//! - CreateQueue:         <https://docs.aws.amazon.com/AWSSimpleQueueService/latest/APIReference/API_CreateQueue.html>
//! - ListQueues:          <https://docs.aws.amazon.com/AWSSimpleQueueService/latest/APIReference/API_ListQueues.html>
//! - DeleteQueue:         <https://docs.aws.amazon.com/AWSSimpleQueueService/latest/APIReference/API_DeleteQueue.html>
//! - GetQueueUrl:         <https://docs.aws.amazon.com/AWSSimpleQueueService/latest/APIReference/API_GetQueueUrl.html>
//! - GetQueueAttributes:  <https://docs.aws.amazon.com/AWSSimpleQueueService/latest/APIReference/API_GetQueueAttributes.html>
//! - SendMessage:         <https://docs.aws.amazon.com/AWSSimpleQueueService/latest/APIReference/API_SendMessage.html>
//! - ReceiveMessage:      <https://docs.aws.amazon.com/AWSSimpleQueueService/latest/APIReference/API_ReceiveMessage.html>
//! - DeleteMessage:       <https://docs.aws.amazon.com/AWSSimpleQueueService/latest/APIReference/API_DeleteMessage.html>
//! - PurgeQueue:          <https://docs.aws.amazon.com/AWSSimpleQueueService/latest/APIReference/API_PurgeQueue.html>

mod common;

use aws_sdk_sqs::types::QueueAttributeName;

#[tokio::test]
async fn e2e_sqs_create_returns_queue_url() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let sqs = aws_sdk_sqs::Client::new(&cfg);

    let q = sqs.create_queue().queue_name("q1").send().await.unwrap();
    let url = q.queue_url().unwrap();
    assert!(url.contains("q1"), "QueueUrl must contain the queue name");
}

#[tokio::test]
async fn e2e_sqs_get_queue_url_for_existing_queue() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let sqs = aws_sdk_sqs::Client::new(&cfg);

    sqs.create_queue().queue_name("known").send().await.unwrap();
    let got = sqs
        .get_queue_url()
        .queue_name("known")
        .send()
        .await
        .unwrap();
    assert!(got.queue_url().unwrap().contains("known"));
}

#[tokio::test]
async fn e2e_sqs_get_queue_url_missing_returns_error() {
    // AWS: NonExistentQueue error when the queue does not exist.
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let sqs = aws_sdk_sqs::Client::new(&cfg);

    let err = sqs.get_queue_url().queue_name("ghost").send().await;
    assert!(err.is_err(), "GetQueueUrl on missing queue must fail");
}

#[tokio::test]
async fn e2e_sqs_list_queues_with_prefix() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let sqs = aws_sdk_sqs::Client::new(&cfg);

    for n in ["alpha", "alpaca", "beta"] {
        sqs.create_queue().queue_name(n).send().await.unwrap();
    }
    let res = sqs
        .list_queues()
        .queue_name_prefix("alp")
        .send()
        .await
        .unwrap();
    assert_eq!(res.queue_urls().len(), 2);
}

#[tokio::test]
async fn e2e_sqs_send_then_receive_message_body() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let sqs = aws_sdk_sqs::Client::new(&cfg);

    let q = sqs.create_queue().queue_name("q").send().await.unwrap();
    let url = q.queue_url().unwrap().to_string();

    let sent = sqs
        .send_message()
        .queue_url(&url)
        .message_body("ping")
        .send()
        .await
        .unwrap();
    // AWS spec: SendMessage returns MessageId and MD5OfMessageBody.
    assert!(sent.message_id().is_some());
    // MD5("ping") = df911f0151f9ef021d410b4be5060972
    assert_eq!(
        sent.md5_of_message_body(),
        Some("df911f0151f9ef021d410b4be5060972")
    );

    let r = sqs.receive_message().queue_url(&url).send().await.unwrap();
    assert_eq!(r.messages().len(), 1);
    assert_eq!(r.messages()[0].body(), Some("ping"));
}

#[tokio::test]
async fn e2e_sqs_receive_message_max_clamped_to_10() {
    // Default MaxNumberOfMessages = 1, valid range 1..=10.
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let sqs = aws_sdk_sqs::Client::new(&cfg);

    let q = sqs.create_queue().queue_name("q").send().await.unwrap();
    let url = q.queue_url().unwrap().to_string();

    for i in 0..15 {
        sqs.send_message()
            .queue_url(&url)
            .message_body(format!("m{i}"))
            .send()
            .await
            .unwrap();
    }
    let r = sqs
        .receive_message()
        .queue_url(&url)
        .max_number_of_messages(10)
        .send()
        .await
        .unwrap();
    assert_eq!(r.messages().len(), 10);
}

#[tokio::test]
async fn e2e_sqs_delete_message_removes_from_inflight() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let sqs = aws_sdk_sqs::Client::new(&cfg);

    let q = sqs.create_queue().queue_name("q").send().await.unwrap();
    let url = q.queue_url().unwrap().to_string();
    sqs.send_message()
        .queue_url(&url)
        .message_body("once")
        .send()
        .await
        .unwrap();

    let r = sqs.receive_message().queue_url(&url).send().await.unwrap();
    let receipt = r.messages()[0].receipt_handle().unwrap().to_string();
    sqs.delete_message()
        .queue_url(&url)
        .receipt_handle(receipt)
        .send()
        .await
        .unwrap();

    let attrs = sqs
        .get_queue_attributes()
        .queue_url(&url)
        .attribute_names(QueueAttributeName::ApproximateNumberOfMessages)
        .attribute_names(QueueAttributeName::ApproximateNumberOfMessagesNotVisible)
        .send()
        .await
        .unwrap();
    let a = attrs.attributes().unwrap();
    assert_eq!(
        a.get(&QueueAttributeName::ApproximateNumberOfMessages)
            .map(String::as_str),
        Some("0")
    );
    assert_eq!(
        a.get(&QueueAttributeName::ApproximateNumberOfMessagesNotVisible)
            .map(String::as_str),
        Some("0")
    );
}

#[tokio::test]
async fn e2e_sqs_purge_queue_empties() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let sqs = aws_sdk_sqs::Client::new(&cfg);

    let q = sqs.create_queue().queue_name("q").send().await.unwrap();
    let url = q.queue_url().unwrap().to_string();
    for i in 0..3 {
        sqs.send_message()
            .queue_url(&url)
            .message_body(format!("m{i}"))
            .send()
            .await
            .unwrap();
    }
    sqs.purge_queue().queue_url(&url).send().await.unwrap();
    let r = sqs.receive_message().queue_url(&url).send().await.unwrap();
    assert_eq!(r.messages().len(), 0);
}

#[tokio::test]
async fn e2e_sqs_delete_queue_then_url_invalid() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let sqs = aws_sdk_sqs::Client::new(&cfg);

    let q = sqs
        .create_queue()
        .queue_name("doomed")
        .send()
        .await
        .unwrap();
    let url = q.queue_url().unwrap().to_string();
    sqs.delete_queue().queue_url(&url).send().await.unwrap();
    let err = sqs.get_queue_url().queue_name("doomed").send().await;
    assert!(err.is_err(), "queue url lookup must fail after DeleteQueue");
}

#[tokio::test]
async fn e2e_sqs_queue_attributes_arn_format() {
    // AWS GetQueueAttributes returns QueueArn in the form
    // `arn:aws:sqs:<region>:<account>:<name>`.
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let sqs = aws_sdk_sqs::Client::new(&cfg);

    let q = sqs.create_queue().queue_name("arnq").send().await.unwrap();
    let url = q.queue_url().unwrap().to_string();
    let attrs = sqs
        .get_queue_attributes()
        .queue_url(&url)
        .attribute_names(QueueAttributeName::QueueArn)
        .send()
        .await
        .unwrap();
    let arn = attrs
        .attributes()
        .unwrap()
        .get(&QueueAttributeName::QueueArn)
        .cloned()
        .unwrap();
    assert!(arn.starts_with("arn:aws:sqs:"), "QueueArn shape: {arn}");
    assert!(arn.ends_with(":arnq"));
}

#[tokio::test]
async fn e2e_sqs_send_message_batch_returns_ids() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let sqs = aws_sdk_sqs::Client::new(&cfg);

    let q = sqs.create_queue().queue_name("batch").send().await.unwrap();
    let url = q.queue_url().unwrap().to_string();

    use aws_sdk_sqs::types::SendMessageBatchRequestEntry;
    let entries: Vec<_> = (0..3)
        .map(|i| {
            SendMessageBatchRequestEntry::builder()
                .id(format!("id{i}"))
                .message_body(format!("m{i}"))
                .build()
                .unwrap()
        })
        .collect();
    let res = sqs
        .send_message_batch()
        .queue_url(&url)
        .set_entries(Some(entries))
        .send()
        .await
        .unwrap();
    assert_eq!(res.successful().len(), 3);
    assert_eq!(res.failed().len(), 0);
}
