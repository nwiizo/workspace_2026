use leptos::prelude::*;
use uuid::Uuid;

use crate::models::ip_address::{IpAddress, IpAddressWithMeta};
#[cfg(feature = "ssr")]
use crate::models::tag::Tag;

#[server]
pub async fn list_ip_addresses(
    subnet_id: Option<Uuid>,
    status_filter: Option<String>,
    cursor: Option<String>,
    limit: Option<i64>,
) -> Result<Vec<IpAddressWithMeta>, ServerFnError> {
    use super::db::pool;

    let pool = pool()?;
    let limit = limit.unwrap_or(50);

    let ips = if let Some(sid) = subnet_id {
        if let Some(ref cur) = cursor {
            sqlx::query_as!(
                IpAddress,
                r#"SELECT id, address::TEXT as "address!", subnet_id,
                   status::TEXT as "status!", hostname, assigned_to,
                   description, mac_address, created_at, updated_at
                   FROM ip_addresses
                   WHERE subnet_id = $1 AND id < $2
                   ORDER BY address, id DESC LIMIT $3"#,
                sid,
                Uuid::parse_str(cur)
                    .map_err(|e| ServerFnError::new(format!("Invalid cursor: {e}")))?,
                limit
            )
            .fetch_all(&pool)
            .await
            .map_err(|e| ServerFnError::new(format!("database error: {e}")))?
        } else {
            sqlx::query_as!(
                IpAddress,
                r#"SELECT id, address::TEXT as "address!", subnet_id,
                   status::TEXT as "status!", hostname, assigned_to,
                   description, mac_address, created_at, updated_at
                   FROM ip_addresses
                   WHERE subnet_id = $1
                   ORDER BY address, id DESC LIMIT $2"#,
                sid,
                limit
            )
            .fetch_all(&pool)
            .await
            .map_err(|e| ServerFnError::new(format!("database error: {e}")))?
        }
    } else if let Some(ref status) = status_filter {
        sqlx::query_as!(
            IpAddress,
            r#"SELECT id, address::TEXT as "address!", subnet_id,
               status::TEXT as "status!", hostname, assigned_to,
               description, mac_address, created_at, updated_at
               FROM ip_addresses
               WHERE status::TEXT = $1
               ORDER BY address LIMIT $2"#,
            status,
            limit
        )
        .fetch_all(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("database error: {e}")))?
    } else {
        sqlx::query_as!(
            IpAddress,
            r#"SELECT id, address::TEXT as "address!", subnet_id,
               status::TEXT as "status!", hostname, assigned_to,
               description, mac_address, created_at, updated_at
               FROM ip_addresses
               ORDER BY address LIMIT $1"#,
            limit
        )
        .fetch_all(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("database error: {e}")))?
    };

    let mut result = Vec::with_capacity(ips.len());
    for ip in ips {
        let tags = sqlx::query_as!(
            Tag,
            r#"SELECT t.id, t.name, t.color, t.created_at
               FROM tags t
               JOIN ip_tags it ON t.id = it.tag_id
               WHERE it.ip_address_id = $1"#,
            ip.id
        )
        .fetch_all(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("failed to fetch tags: {e}")))?;

        let subnet_cidr: String = sqlx::query_scalar!(
            r#"SELECT cidr::TEXT as "cidr!" FROM subnets WHERE id = $1"#,
            ip.subnet_id
        )
        .fetch_one(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("failed to fetch subnet CIDR: {e}")))?;

        result.push(IpAddressWithMeta {
            ip,
            subnet_cidr,
            tags,
        });
    }

    Ok(result)
}

#[server]
pub async fn get_ip_address(id: Uuid) -> Result<IpAddressWithMeta, ServerFnError> {
    use super::db::pool;

    let pool = pool()?;
    let ip = sqlx::query_as!(
        IpAddress,
        r#"SELECT id, address::TEXT as "address!", subnet_id,
           status::TEXT as "status!", hostname, assigned_to,
           description, mac_address, created_at, updated_at
           FROM ip_addresses WHERE id = $1"#,
        id
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("database error: {e}")))?
    .ok_or_else(|| ServerFnError::new("IP address not found"))?;

    let tags = sqlx::query_as!(
        Tag,
        r#"SELECT t.id, t.name, t.color, t.created_at
           FROM tags t
           JOIN ip_tags it ON t.id = it.tag_id
           WHERE it.ip_address_id = $1"#,
        id
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("failed to fetch tags: {e}")))?;

    let subnet_cidr: String = sqlx::query_scalar!(
        r#"SELECT cidr::TEXT as "cidr!" FROM subnets WHERE id = $1"#,
        ip.subnet_id
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("failed to fetch subnet CIDR: {e}")))?;

    Ok(IpAddressWithMeta {
        ip,
        subnet_cidr,
        tags,
    })
}

#[server]
pub async fn create_ip_address(
    address: String,
    subnet_id: Uuid,
    status: String,
    hostname: Option<String>,
    assigned_to: Option<String>,
    description: Option<String>,
    mac_address: Option<String>,
) -> Result<IpAddress, ServerFnError> {
    use super::db::pool;

    let address = address.trim().to_string();
    if address.is_empty() {
        return Err(ServerFnError::new("IP address cannot be empty"));
    }
    if address.parse::<std::net::IpAddr>().is_err() {
        return Err(ServerFnError::new("invalid IP address format"));
    }

    let valid_statuses = ["available", "assigned", "reserved", "deprecated"];
    if !valid_statuses.contains(&status.as_str()) {
        return Err(ServerFnError::new(format!(
            "invalid status '{}', must be one of: {}",
            status,
            valid_statuses.join(", ")
        )));
    }

    let pool = pool()?;
    let id = Uuid::now_v7();

    let ip: IpAddress = sqlx::query_as(
        r#"INSERT INTO ip_addresses (id, address, subnet_id, status, hostname, assigned_to, description, mac_address)
           VALUES ($1, $2::INET, $3, $4::ip_status, $5, $6, $7, $8)
           RETURNING id, address::TEXT as address, subnet_id,
           status::TEXT as status, hostname, assigned_to,
           description, mac_address, created_at, updated_at"#,
    )
    .bind(id)
    .bind(&address)
    .bind(subnet_id)
    .bind(&status)
    .bind(&hostname)
    .bind(&assigned_to)
    .bind(&description)
    .bind(&mac_address)
    .fetch_one(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("failed to create IP address: {e}")))?;

    if let Err(e) = sqlx::query!(
        "INSERT INTO audit_logs (id, entity_type, entity_id, action, new_value)
         VALUES ($1, 'ip_address', $2, 'create', $3)",
        Uuid::now_v7(),
        id,
        serde_json::to_value(&ip).ok()
    )
    .execute(&pool)
    .await
    {
        eprintln!("failed to write audit log for IP address create: {e}");
    }

    Ok(ip)
}

#[server]
pub async fn update_ip_address(
    id: Uuid,
    status: String,
    hostname: Option<String>,
    assigned_to: Option<String>,
    description: Option<String>,
    mac_address: Option<String>,
) -> Result<IpAddress, ServerFnError> {
    use super::db::pool;

    let valid_statuses = ["available", "assigned", "reserved", "deprecated"];
    if !valid_statuses.contains(&status.as_str()) {
        return Err(ServerFnError::new(format!(
            "invalid status '{}', must be one of: {}",
            status,
            valid_statuses.join(", ")
        )));
    }

    let pool = pool()?;

    let ip: IpAddress = sqlx::query_as(
        r#"UPDATE ip_addresses
           SET status = $2::ip_status, hostname = $3, assigned_to = $4,
               description = $5, mac_address = $6
           WHERE id = $1
           RETURNING id, address::TEXT as address, subnet_id,
           status::TEXT as status, hostname, assigned_to,
           description, mac_address, created_at, updated_at"#,
    )
    .bind(id)
    .bind(&status)
    .bind(&hostname)
    .bind(&assigned_to)
    .bind(&description)
    .bind(&mac_address)
    .fetch_optional(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("failed to update IP address: {e}")))?
    .ok_or_else(|| ServerFnError::new("IP address not found"))?;

    if let Err(e) = sqlx::query!(
        "INSERT INTO audit_logs (id, entity_type, entity_id, action, new_value)
         VALUES ($1, 'ip_address', $2, 'update', $3)",
        Uuid::now_v7(),
        id,
        serde_json::to_value(&ip).ok()
    )
    .execute(&pool)
    .await
    {
        eprintln!("failed to write audit log for IP address update: {e}");
    }

    Ok(ip)
}

#[server]
pub async fn delete_ip_address(id: Uuid) -> Result<(), ServerFnError> {
    use super::db::pool;

    let pool = pool()?;

    if let Err(e) = sqlx::query!(
        "INSERT INTO audit_logs (id, entity_type, entity_id, action)
         VALUES ($1, 'ip_address', $2, 'delete')",
        Uuid::now_v7(),
        id,
    )
    .execute(&pool)
    .await
    {
        eprintln!("failed to write audit log for IP address delete: {e}");
    }

    let result = sqlx::query!("DELETE FROM ip_addresses WHERE id = $1", id)
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("database error: {e}")))?;

    if result.rows_affected() == 0 {
        return Err(ServerFnError::new("IP address not found"));
    }
    Ok(())
}
