//! AWS Backup E2E tests.
//! Refs: <https://docs.aws.amazon.com/aws-backup/latest/devguide/api-reference.html>

mod common;

use aws_sdk_backup::types::{BackupPlanInput, BackupRuleInput};
use pretty_assertions::assert_eq;

#[tokio::test]
async fn e2e_backup_create_vault() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let b = aws_sdk_backup::Client::new(&cfg);
    let res = b
        .create_backup_vault()
        .backup_vault_name("v1")
        .send()
        .await
        .unwrap();
    assert_eq!(res.backup_vault_name(), Some("v1"));
}

#[tokio::test]
async fn e2e_backup_describe_vault() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let b = aws_sdk_backup::Client::new(&cfg);
    b.create_backup_vault()
        .backup_vault_name("v2")
        .send()
        .await
        .unwrap();
    let res = b
        .describe_backup_vault()
        .backup_vault_name("v2")
        .send()
        .await
        .unwrap();
    assert_eq!(res.backup_vault_name(), Some("v2"));
    assert_eq!(res.number_of_recovery_points(), 0);
}

#[tokio::test]
async fn e2e_backup_duplicate_vault_fails() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let b = aws_sdk_backup::Client::new(&cfg);
    b.create_backup_vault()
        .backup_vault_name("dup")
        .send()
        .await
        .unwrap();
    let err = b
        .create_backup_vault()
        .backup_vault_name("dup")
        .send()
        .await;
    assert!(err.is_err());
}

#[tokio::test]
async fn e2e_backup_create_plan() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let b = aws_sdk_backup::Client::new(&cfg);
    let rule = BackupRuleInput::builder()
        .rule_name("daily-rule")
        .target_backup_vault_name("v")
        .schedule_expression("cron(0 5 ? * * *)")
        .build()
        .unwrap();
    let plan = BackupPlanInput::builder()
        .backup_plan_name("daily")
        .rules(rule)
        .build()
        .unwrap();
    let res = b
        .create_backup_plan()
        .backup_plan(plan)
        .send()
        .await
        .unwrap();
    assert!(res.backup_plan_id().is_some());
}

#[tokio::test]
async fn e2e_backup_list_vaults_then_delete() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let b = aws_sdk_backup::Client::new(&cfg);
    for n in ["a", "b"] {
        b.create_backup_vault()
            .backup_vault_name(n)
            .send()
            .await
            .unwrap();
    }
    let res = b.list_backup_vaults().send().await.unwrap();
    assert_eq!(res.backup_vault_list().len(), 2);
    b.delete_backup_vault()
        .backup_vault_name("a")
        .send()
        .await
        .unwrap();
    let res2 = b.list_backup_vaults().send().await.unwrap();
    assert_eq!(res2.backup_vault_list().len(), 1);
}
