use leptos::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub entity_type: String,
    pub id: String,
    pub title: String,
    pub description: String,
    pub similarity: f32,
}

#[server]
pub async fn global_search(query: String) -> Result<Vec<SearchResult>, ServerFnError> {
    use super::db::pool;

    let pool = pool()?;
    let query = query.trim().to_string();
    if query.is_empty() {
        return Ok(Vec::new());
    }

    let mut results = Vec::new();

    // Search subnets
    let subnet_results = sqlx::query!(
        r#"SELECT id, name, cidr::TEXT as "cidr!",
           similarity(name, $1) as "sim!"
           FROM subnets
           WHERE name % $1 OR cidr::TEXT LIKE '%' || $1 || '%'
           ORDER BY similarity(name, $1) DESC
           LIMIT 10"#,
        query
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("failed to search subnets: {e}")))?;

    for row in subnet_results {
        results.push(SearchResult {
            entity_type: "subnet".to_string(),
            id: row.id.to_string(),
            title: format!("{} ({})", row.name, row.cidr),
            description: "Subnet".to_string(),
            similarity: row.sim,
        });
    }

    // Search IP addresses
    let ip_results = sqlx::query!(
        r#"SELECT id, address::TEXT as "address!",
           hostname, assigned_to,
           GREATEST(
               COALESCE(similarity(hostname, $1), 0),
               COALESCE(similarity(assigned_to, $1), 0)
           ) as "sim!"
           FROM ip_addresses
           WHERE hostname % $1
              OR assigned_to % $1
              OR address::TEXT LIKE '%' || $1 || '%'
           ORDER BY "sim!" DESC
           LIMIT 10"#,
        query
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("failed to search IP addresses: {e}")))?;

    for row in ip_results {
        results.push(SearchResult {
            entity_type: "ip_address".to_string(),
            id: row.id.to_string(),
            title: row.address,
            description: format!(
                "{}{}",
                row.hostname.unwrap_or_default(),
                row.assigned_to
                    .map(|a| format!(" ({})", a))
                    .unwrap_or_default()
            ),
            similarity: row.sim,
        });
    }

    // Search VLANs
    let vlan_results = sqlx::query!(
        r#"SELECT id, vlan_id, name,
           similarity(name, $1) as "sim!"
           FROM vlans
           WHERE name % $1
           ORDER BY similarity(name, $1) DESC
           LIMIT 10"#,
        query
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("failed to search VLANs: {e}")))?;

    for row in vlan_results {
        results.push(SearchResult {
            entity_type: "vlan".to_string(),
            id: row.id.to_string(),
            title: format!("VLAN {} - {}", row.vlan_id, row.name),
            description: "VLAN".to_string(),
            similarity: row.sim,
        });
    }

    // Search DNS records
    let dns_results = sqlx::query!(
        r#"SELECT id, hostname, record_type::TEXT as "record_type!",
           similarity(hostname, $1) as "sim!"
           FROM dns_records
           WHERE hostname % $1
           ORDER BY similarity(hostname, $1) DESC
           LIMIT 10"#,
        query
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("failed to search DNS records: {e}")))?;

    for row in dns_results {
        results.push(SearchResult {
            entity_type: "dns_record".to_string(),
            id: row.id.to_string(),
            title: row.hostname,
            description: format!("{} record", row.record_type),
            similarity: row.sim,
        });
    }

    results.sort_by(|a, b| {
        b.similarity
            .partial_cmp(&a.similarity)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    results.truncate(20);

    Ok(results)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardStats {
    pub total_subnets: i64,
    pub total_ips: i64,
    pub assigned_ips: i64,
    pub total_vlans: i64,
    pub active_alerts: i64,
    pub avg_utilization: f64,
}

#[server]
pub async fn get_dashboard_stats() -> Result<DashboardStats, ServerFnError> {
    use super::db::pool;

    let pool = pool()?;

    let total_subnets = sqlx::query_scalar!(r#"SELECT COUNT(*) as "count!" FROM subnets"#)
        .fetch_one(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("failed to count subnets: {e}")))?;

    let total_ips = sqlx::query_scalar!(r#"SELECT COUNT(*) as "count!" FROM ip_addresses"#)
        .fetch_one(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("failed to count IP addresses: {e}")))?;

    let assigned_ips = sqlx::query_scalar!(
        r#"SELECT COUNT(*) as "count!" FROM ip_addresses WHERE status != 'available'"#
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("failed to count assigned IPs: {e}")))?;

    let total_vlans = sqlx::query_scalar!(r#"SELECT COUNT(*) as "count!" FROM vlans"#)
        .fetch_one(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("failed to count VLANs: {e}")))?;

    let active_alerts =
        sqlx::query_scalar!(r#"SELECT COUNT(*) as "count!" FROM alerts WHERE NOT is_acknowledged"#)
            .fetch_one(&pool)
            .await
            .map_err(|e| ServerFnError::new(format!("failed to count alerts: {e}")))?;

    let avg_utilization = sqlx::query_scalar!(
        r#"SELECT COALESCE(AVG(
               CASE WHEN total_addresses > 0
               THEN (used_count::float / total_addresses::float) * 100
               ELSE 0 END
           ), 0) as "avg!" FROM subnets"#
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("failed to calculate utilization: {e}")))?;

    Ok(DashboardStats {
        total_subnets,
        total_ips,
        assigned_ips,
        total_vlans,
        active_alerts,
        avg_utilization,
    })
}
