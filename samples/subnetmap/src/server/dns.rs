use leptos::prelude::*;
use uuid::Uuid;

use crate::models::dns_record::DnsRecord;

#[server]
pub async fn list_dns_records(
    ip_address_id: Option<Uuid>,
) -> Result<Vec<DnsRecord>, ServerFnError> {
    use super::db::pool;

    let pool = pool()?;
    let rows = if let Some(ip_id) = ip_address_id {
        sqlx::query_as!(
            DnsRecord,
            r#"SELECT id, record_type::TEXT as "record_type!", hostname,
               ip_address_id, created_at, updated_at
               FROM dns_records WHERE ip_address_id = $1
               ORDER BY hostname"#,
            ip_id
        )
        .fetch_all(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("database error: {e}")))?
    } else {
        sqlx::query_as!(
            DnsRecord,
            r#"SELECT id, record_type::TEXT as "record_type!", hostname,
               ip_address_id, created_at, updated_at
               FROM dns_records ORDER BY hostname LIMIT 100"#
        )
        .fetch_all(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("database error: {e}")))?
    };

    Ok(rows)
}

#[server]
pub async fn create_dns_record(
    record_type: String,
    hostname: String,
    ip_address_id: Uuid,
) -> Result<DnsRecord, ServerFnError> {
    use super::db::pool;

    let hostname = hostname.trim().to_string();
    if hostname.is_empty() {
        return Err(ServerFnError::new("hostname cannot be empty"));
    }

    let valid_record_types = ["A", "AAAA", "PTR"];
    if !valid_record_types.contains(&record_type.as_str()) {
        return Err(ServerFnError::new(format!(
            "invalid record type '{}', must be one of: {}",
            record_type,
            valid_record_types.join(", ")
        )));
    }

    let pool = pool()?;
    let id = Uuid::now_v7();

    let record: DnsRecord = sqlx::query_as(
        r#"INSERT INTO dns_records (id, record_type, hostname, ip_address_id)
           VALUES ($1, $2::dns_record_type, $3, $4)
           RETURNING id, record_type::TEXT as record_type, hostname,
           ip_address_id, created_at, updated_at"#,
    )
    .bind(id)
    .bind(&record_type)
    .bind(&hostname)
    .bind(ip_address_id)
    .fetch_one(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("failed to create DNS record: {e}")))?;

    if let Err(e) = sqlx::query!(
        "INSERT INTO audit_logs (id, entity_type, entity_id, action, new_value)
         VALUES ($1, 'dns_record', $2, 'create', $3)",
        Uuid::now_v7(),
        id,
        serde_json::to_value(&record).ok()
    )
    .execute(&pool)
    .await
    {
        eprintln!("failed to write audit log for DNS record create: {e}");
    }

    Ok(record)
}

#[server]
pub async fn delete_dns_record(id: Uuid) -> Result<(), ServerFnError> {
    use super::db::pool;

    let pool = pool()?;
    let result = sqlx::query!("DELETE FROM dns_records WHERE id = $1", id)
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("database error: {e}")))?;

    if result.rows_affected() == 0 {
        return Err(ServerFnError::new("DNS record not found"));
    }
    Ok(())
}
