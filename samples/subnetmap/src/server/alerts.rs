use leptos::prelude::*;
use uuid::Uuid;

use crate::models::alert::Alert;

#[server]
pub async fn list_alerts(unacknowledged_only: Option<bool>) -> Result<Vec<Alert>, ServerFnError> {
    use super::db::pool;

    let pool = pool()?;
    let rows = if unacknowledged_only.unwrap_or(false) {
        sqlx::query_as!(
            Alert,
            r#"SELECT id, alert_type::TEXT as "alert_type!", subnet_id,
               message, is_acknowledged, created_at
               FROM alerts
               WHERE NOT is_acknowledged
               ORDER BY created_at DESC"#
        )
        .fetch_all(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("failed to query alerts: {e}")))?
    } else {
        sqlx::query_as!(
            Alert,
            r#"SELECT id, alert_type::TEXT as "alert_type!", subnet_id,
               message, is_acknowledged, created_at
               FROM alerts
               ORDER BY created_at DESC LIMIT 100"#
        )
        .fetch_all(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("failed to query alerts: {e}")))?
    };

    Ok(rows)
}

#[server]
pub async fn acknowledge_alert(id: Uuid) -> Result<(), ServerFnError> {
    use super::db::pool;

    let pool = pool()?;
    sqlx::query!("UPDATE alerts SET is_acknowledged = TRUE WHERE id = $1", id)
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("failed to query alerts: {e}")))?;

    Ok(())
}

#[server]
pub async fn check_utilization_alerts() -> Result<Vec<Alert>, ServerFnError> {
    use super::db::pool;

    let pool = pool()?;

    // Find subnets with > 80% utilization
    let high_util = sqlx::query!(
        r#"SELECT id, cidr::TEXT as "cidr!", name, used_count, total_addresses
           FROM subnets
           WHERE total_addresses > 0
           AND (used_count::float / total_addresses::float) > 0.8"#
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("failed to query alerts: {e}")))?;

    let mut alerts = Vec::new();
    for subnet in high_util {
        let pct = (subnet.used_count as f64 / subnet.total_addresses as f64 * 100.0) as i32;
        let alert_type = if pct >= 100 {
            "subnet_full"
        } else {
            "high_utilization"
        };
        let message = format!(
            "Subnet {} ({}) is at {}% utilization ({}/{})",
            subnet.name, subnet.cidr, pct, subnet.used_count, subnet.total_addresses
        );

        // Only create if no active alert exists for this subnet
        let existing = sqlx::query_scalar!(
            r#"SELECT COUNT(*) as "count!" FROM alerts
               WHERE subnet_id = $1 AND NOT is_acknowledged
               AND alert_type::TEXT = $2"#,
            subnet.id,
            alert_type
        )
        .fetch_one(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("failed to check existing alerts: {e}")))?;

        if existing == 0 {
            let id = Uuid::now_v7();
            let alert: Alert = sqlx::query_as(
                r#"INSERT INTO alerts (id, alert_type, subnet_id, message)
                   VALUES ($1, $2::alert_type, $3, $4)
                   RETURNING id, alert_type::TEXT as alert_type, subnet_id,
                   message, is_acknowledged, created_at"#,
            )
            .bind(id)
            .bind(alert_type)
            .bind(subnet.id)
            .bind(&message)
            .fetch_one(&pool)
            .await
            .map_err(|e| ServerFnError::new(format!("failed to query alerts: {e}")))?;

            alerts.push(alert);
        }
    }

    Ok(alerts)
}
