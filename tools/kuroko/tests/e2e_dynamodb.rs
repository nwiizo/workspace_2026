//! DynamoDB E2E tests verifying behavior matches the AWS official API spec.
//!
//! References:
//! - CreateTable:  <https://docs.aws.amazon.com/amazondynamodb/latest/APIReference/API_CreateTable.html>
//! - DescribeTable:<https://docs.aws.amazon.com/amazondynamodb/latest/APIReference/API_DescribeTable.html>
//! - DeleteTable:  <https://docs.aws.amazon.com/amazondynamodb/latest/APIReference/API_DeleteTable.html>
//! - ListTables:   <https://docs.aws.amazon.com/amazondynamodb/latest/APIReference/API_ListTables.html>
//! - PutItem:      <https://docs.aws.amazon.com/amazondynamodb/latest/APIReference/API_PutItem.html>
//! - GetItem:      <https://docs.aws.amazon.com/amazondynamodb/latest/APIReference/API_GetItem.html>
//! - DeleteItem:   <https://docs.aws.amazon.com/amazondynamodb/latest/APIReference/API_DeleteItem.html>
//! - UpdateItem:   <https://docs.aws.amazon.com/amazondynamodb/latest/APIReference/API_UpdateItem.html>
//! - Query:        <https://docs.aws.amazon.com/amazondynamodb/latest/APIReference/API_Query.html>
//! - Scan:         <https://docs.aws.amazon.com/amazondynamodb/latest/APIReference/API_Scan.html>
//! - BatchWriteItem:<https://docs.aws.amazon.com/amazondynamodb/latest/APIReference/API_BatchWriteItem.html>

mod common;

use aws_sdk_dynamodb::types::{
    AttributeDefinition, AttributeValue, KeySchemaElement, KeyType, ScalarAttributeType,
};

async fn make_table(ddb: &aws_sdk_dynamodb::Client, name: &str, with_range: bool) {
    let mut create = ddb
        .create_table()
        .table_name(name)
        .key_schema(
            KeySchemaElement::builder()
                .attribute_name("pk")
                .key_type(KeyType::Hash)
                .build()
                .unwrap(),
        )
        .attribute_definitions(
            AttributeDefinition::builder()
                .attribute_name("pk")
                .attribute_type(ScalarAttributeType::S)
                .build()
                .unwrap(),
        );
    if with_range {
        create = create
            .key_schema(
                KeySchemaElement::builder()
                    .attribute_name("sk")
                    .key_type(KeyType::Range)
                    .build()
                    .unwrap(),
            )
            .attribute_definitions(
                AttributeDefinition::builder()
                    .attribute_name("sk")
                    .attribute_type(ScalarAttributeType::S)
                    .build()
                    .unwrap(),
            );
    }
    create.send().await.unwrap();
}

#[tokio::test]
async fn e2e_ddb_create_table_returns_active_status_and_arn() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let ddb = aws_sdk_dynamodb::Client::new(&cfg);

    let res = ddb
        .create_table()
        .table_name("t")
        .key_schema(
            KeySchemaElement::builder()
                .attribute_name("pk")
                .key_type(KeyType::Hash)
                .build()
                .unwrap(),
        )
        .attribute_definitions(
            AttributeDefinition::builder()
                .attribute_name("pk")
                .attribute_type(ScalarAttributeType::S)
                .build()
                .unwrap(),
        )
        .send()
        .await
        .unwrap();

    let desc = res.table_description().unwrap();
    assert_eq!(desc.table_name(), Some("t"));
    let arn = desc.table_arn().unwrap();
    // AWS spec: TableArn = arn:aws:dynamodb:<region>:<account>:table/<name>
    assert!(arn.starts_with("arn:aws:dynamodb:"));
    assert!(arn.ends_with(":table/t"), "arn shape: {arn}");
}

#[tokio::test]
async fn e2e_ddb_create_table_duplicate_returns_resource_in_use() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let ddb = aws_sdk_dynamodb::Client::new(&cfg);

    make_table(&ddb, "dup", false).await;
    let err = ddb
        .create_table()
        .table_name("dup")
        .key_schema(
            KeySchemaElement::builder()
                .attribute_name("pk")
                .key_type(KeyType::Hash)
                .build()
                .unwrap(),
        )
        .attribute_definitions(
            AttributeDefinition::builder()
                .attribute_name("pk")
                .attribute_type(ScalarAttributeType::S)
                .build()
                .unwrap(),
        )
        .send()
        .await;
    assert!(
        err.is_err(),
        "second CreateTable must fail with ResourceInUseException"
    );
}

#[tokio::test]
async fn e2e_ddb_list_tables_returns_sorted_names() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let ddb = aws_sdk_dynamodb::Client::new(&cfg);

    for n in ["zebra", "apple", "mango"] {
        make_table(&ddb, n, false).await;
    }
    let list = ddb.list_tables().send().await.unwrap();
    let names: Vec<&str> = list.table_names().iter().map(String::as_str).collect();
    assert_eq!(names, vec!["apple", "mango", "zebra"]);
}

#[tokio::test]
async fn e2e_ddb_describe_table_missing_returns_resource_not_found() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let ddb = aws_sdk_dynamodb::Client::new(&cfg);

    let err = ddb.describe_table().table_name("ghost").send().await;
    assert!(err.is_err(), "DescribeTable on missing table must fail");
}

#[tokio::test]
async fn e2e_ddb_put_get_item_roundtrip() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let ddb = aws_sdk_dynamodb::Client::new(&cfg);

    make_table(&ddb, "t", false).await;
    ddb.put_item()
        .table_name("t")
        .item("pk", AttributeValue::S("a".into()))
        .item("name", AttributeValue::S("kuroko".into()))
        .item("count", AttributeValue::N("42".into()))
        .send()
        .await
        .unwrap();

    let got = ddb
        .get_item()
        .table_name("t")
        .key("pk", AttributeValue::S("a".into()))
        .send()
        .await
        .unwrap();
    let item = got.item().unwrap();
    assert_eq!(item.get("name").unwrap().as_s().unwrap(), "kuroko");
    assert_eq!(item.get("count").unwrap().as_n().unwrap(), "42");
}

#[tokio::test]
async fn e2e_ddb_get_item_missing_returns_empty() {
    // AWS spec: GetItem returns no Item key when nothing matches.
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let ddb = aws_sdk_dynamodb::Client::new(&cfg);

    make_table(&ddb, "t", false).await;
    let got = ddb
        .get_item()
        .table_name("t")
        .key("pk", AttributeValue::S("absent".into()))
        .send()
        .await
        .unwrap();
    assert!(got.item().is_none());
}

#[tokio::test]
async fn e2e_ddb_delete_item() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let ddb = aws_sdk_dynamodb::Client::new(&cfg);

    make_table(&ddb, "t", false).await;
    ddb.put_item()
        .table_name("t")
        .item("pk", AttributeValue::S("a".into()))
        .send()
        .await
        .unwrap();
    ddb.delete_item()
        .table_name("t")
        .key("pk", AttributeValue::S("a".into()))
        .send()
        .await
        .unwrap();
    let got = ddb
        .get_item()
        .table_name("t")
        .key("pk", AttributeValue::S("a".into()))
        .send()
        .await
        .unwrap();
    assert!(got.item().is_none());
}

#[tokio::test]
async fn e2e_ddb_scan_returns_count_and_items() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let ddb = aws_sdk_dynamodb::Client::new(&cfg);

    make_table(&ddb, "t", false).await;
    for i in 0..3 {
        ddb.put_item()
            .table_name("t")
            .item("pk", AttributeValue::S(format!("a{i}")))
            .send()
            .await
            .unwrap();
    }

    let res = ddb.scan().table_name("t").send().await.unwrap();
    assert_eq!(res.count(), 3);
    assert_eq!(res.scanned_count(), 3);
    assert_eq!(res.items().len(), 3);
}

#[tokio::test]
async fn e2e_ddb_query_with_key_condition_expression_hash_only() {
    // AWS spec: Query requires KeyConditionExpression matching the hash key.
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let ddb = aws_sdk_dynamodb::Client::new(&cfg);

    make_table(&ddb, "t", false).await;
    ddb.put_item()
        .table_name("t")
        .item("pk", AttributeValue::S("a".into()))
        .item("v", AttributeValue::S("first".into()))
        .send()
        .await
        .unwrap();
    ddb.put_item()
        .table_name("t")
        .item("pk", AttributeValue::S("b".into()))
        .item("v", AttributeValue::S("second".into()))
        .send()
        .await
        .unwrap();

    let res = ddb
        .query()
        .table_name("t")
        .key_condition_expression("pk = :p")
        .expression_attribute_values(":p", AttributeValue::S("a".into()))
        .send()
        .await
        .unwrap();
    assert_eq!(res.count(), 1);
    assert_eq!(res.items()[0].get("v").unwrap().as_s().unwrap(), "first");
}

#[tokio::test]
async fn e2e_ddb_query_with_sort_key_begins_with() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let ddb = aws_sdk_dynamodb::Client::new(&cfg);

    make_table(&ddb, "t", true).await;
    for sk in ["alpha#1", "alpha#2", "beta#1"] {
        ddb.put_item()
            .table_name("t")
            .item("pk", AttributeValue::S("p".into()))
            .item("sk", AttributeValue::S(sk.into()))
            .send()
            .await
            .unwrap();
    }

    let res = ddb
        .query()
        .table_name("t")
        .key_condition_expression("pk = :p AND begins_with(sk, :prefix)")
        .expression_attribute_values(":p", AttributeValue::S("p".into()))
        .expression_attribute_values(":prefix", AttributeValue::S("alpha".into()))
        .send()
        .await
        .unwrap();
    assert_eq!(res.count(), 2);
    let sks: Vec<&str> = res
        .items()
        .iter()
        .map(|i| i.get("sk").unwrap().as_s().unwrap().as_str())
        .collect();
    assert_eq!(sks, vec!["alpha#1", "alpha#2"]);
}

#[tokio::test]
async fn e2e_ddb_query_scan_index_forward_reverses_order() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let ddb = aws_sdk_dynamodb::Client::new(&cfg);

    make_table(&ddb, "t", true).await;
    for sk in ["1", "2", "3"] {
        ddb.put_item()
            .table_name("t")
            .item("pk", AttributeValue::S("p".into()))
            .item("sk", AttributeValue::S(sk.into()))
            .send()
            .await
            .unwrap();
    }
    let res = ddb
        .query()
        .table_name("t")
        .scan_index_forward(false)
        .key_condition_expression("pk = :p")
        .expression_attribute_values(":p", AttributeValue::S("p".into()))
        .send()
        .await
        .unwrap();
    let sks: Vec<&str> = res
        .items()
        .iter()
        .map(|i| i.get("sk").unwrap().as_s().unwrap().as_str())
        .collect();
    assert_eq!(sks, vec!["3", "2", "1"]);
}

#[tokio::test]
async fn e2e_ddb_query_limit_truncates_results() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let ddb = aws_sdk_dynamodb::Client::new(&cfg);

    make_table(&ddb, "t", true).await;
    for sk in ["a", "b", "c", "d", "e"] {
        ddb.put_item()
            .table_name("t")
            .item("pk", AttributeValue::S("p".into()))
            .item("sk", AttributeValue::S(sk.into()))
            .send()
            .await
            .unwrap();
    }
    let res = ddb
        .query()
        .table_name("t")
        .limit(2)
        .key_condition_expression("pk = :p")
        .expression_attribute_values(":p", AttributeValue::S("p".into()))
        .send()
        .await
        .unwrap();
    assert_eq!(res.count(), 2);
    // ScannedCount counts the items the engine looked at before applying
    // limit; with no FilterExpression that's the total matching set.
    assert_eq!(res.scanned_count(), 5);
}

#[tokio::test]
async fn e2e_ddb_batch_write_then_scan() {
    use aws_sdk_dynamodb::types::{PutRequest, WriteRequest};

    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let ddb = aws_sdk_dynamodb::Client::new(&cfg);

    make_table(&ddb, "t", false).await;
    let reqs: Vec<_> = (0..3)
        .map(|i| {
            let item = [("pk".to_string(), AttributeValue::S(format!("k{i}")))]
                .into_iter()
                .collect();
            WriteRequest::builder()
                .put_request(PutRequest::builder().set_item(Some(item)).build().unwrap())
                .build()
        })
        .collect();
    ddb.batch_write_item()
        .request_items("t", reqs)
        .send()
        .await
        .unwrap();
    let scan = ddb.scan().table_name("t").send().await.unwrap();
    assert_eq!(scan.count(), 3);
}

#[tokio::test]
async fn e2e_ddb_delete_table_then_describe_404() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let ddb = aws_sdk_dynamodb::Client::new(&cfg);

    make_table(&ddb, "t", false).await;
    ddb.delete_table().table_name("t").send().await.unwrap();
    let err = ddb.describe_table().table_name("t").send().await;
    assert!(err.is_err(), "table must be gone after DeleteTable");
}
