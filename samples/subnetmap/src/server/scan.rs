use leptos::prelude::*;
use uuid::Uuid;

use crate::models::scan::ScanResult;

#[server]
pub async fn list_scan_results(subnet_id: Uuid) -> Result<Vec<ScanResult>, ServerFnError> {
    use super::db::pool;

    let pool = pool()?;
    let rows = sqlx::query_as!(
        ScanResult,
        r#"SELECT id, subnet_id, ip_address::TEXT as "ip_address!", is_alive,
           last_seen, created_at
           FROM scan_results
           WHERE subnet_id = $1
           ORDER BY ip_address"#,
        subnet_id
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("database error: {e}")))?;

    Ok(rows)
}

#[server]
pub async fn run_scan(subnet_id: Uuid) -> Result<Vec<ScanResult>, ServerFnError> {
    use super::db::pool;
    use ipnetwork::IpNetwork;
    use std::net::IpAddr;

    let pool = pool()?;

    let cidr: String = sqlx::query_scalar!(
        r#"SELECT cidr::TEXT as "cidr!" FROM subnets WHERE id = $1"#,
        subnet_id
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("database error: {e}")))?
    .ok_or_else(|| ServerFnError::new("Subnet not found"))?;

    let network: IpNetwork = cidr
        .parse()
        .map_err(|e| ServerFnError::new(format!("Invalid CIDR: {e}")))?;

    // Limit to first 256 addresses to prevent long-running scans on large subnets
    const MAX_SCAN_HOSTS: usize = 256;

    let mut results = Vec::new();
    let hosts: Vec<IpAddr> = network.iter().take(MAX_SCAN_HOSTS).collect();

    for addr in hosts {
        let addr_str = addr.to_string();
        let is_alive = tokio::process::Command::new("ping")
            .args(["-c", "1", "-W", "1", &addr_str])
            .output()
            .await
            .map(|o| o.status.success())
            .unwrap_or(false);

        let now = chrono::Utc::now().naive_utc();
        let id = Uuid::now_v7();

        let scan: ScanResult = sqlx::query_as(
            r#"INSERT INTO scan_results (id, subnet_id, ip_address, is_alive, last_seen)
               VALUES ($1, $2, $3::INET, $4, CASE WHEN $4 THEN $5 ELSE NULL END)
               ON CONFLICT (subnet_id, ip_address)
               DO UPDATE SET is_alive = $4, last_seen = CASE WHEN $4 THEN $5 ELSE scan_results.last_seen END
               RETURNING id, subnet_id, ip_address::TEXT as ip_address, is_alive,
               last_seen, created_at"#,
        )
        .bind(id)
        .bind(subnet_id)
        .bind(&addr_str)
        .bind(is_alive)
        .bind(now)
        .fetch_one(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("database error: {e}")))?;

        results.push(scan);
    }

    if let Err(e) = sqlx::query!(
        "INSERT INTO audit_logs (id, entity_type, entity_id, action, new_value)
         VALUES ($1, 'scan', $2, 'scan', $3)",
        Uuid::now_v7(),
        subnet_id,
        serde_json::json!({ "total": results.len(), "alive": results.iter().filter(|r| r.is_alive).count() })
    )
    .execute(&pool)
    .await
    {
        eprintln!("failed to write audit log for scan: {e}");
    }

    Ok(results)
}
