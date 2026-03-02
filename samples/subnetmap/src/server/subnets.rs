use leptos::prelude::*;
use uuid::Uuid;

use crate::models::subnet::{Subnet, SubnetWithChildren};

#[server]
pub async fn list_subnets() -> Result<Vec<Subnet>, ServerFnError> {
    use super::db::pool;

    let pool = pool()?;
    let rows = sqlx::query_as!(
        Subnet,
        r#"SELECT id, cidr::TEXT as "cidr!", name, description, parent_id,
           used_count, total_addresses, created_at, updated_at
           FROM subnets ORDER BY cidr"#
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("database error: {e}")))?;

    Ok(rows)
}

#[server]
pub async fn get_subnet(id: Uuid) -> Result<SubnetWithChildren, ServerFnError> {
    use super::db::pool;

    let pool = pool()?;
    let subnet = sqlx::query_as!(
        Subnet,
        r#"SELECT id, cidr::TEXT as "cidr!", name, description, parent_id,
           used_count, total_addresses, created_at, updated_at
           FROM subnets WHERE id = $1"#,
        id
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("database error: {e}")))?
    .ok_or_else(|| ServerFnError::new("Subnet not found"))?;

    let children = sqlx::query_as!(
        Subnet,
        r#"SELECT id, cidr::TEXT as "cidr!", name, description, parent_id,
           used_count, total_addresses, created_at, updated_at
           FROM subnets WHERE parent_id = $1 ORDER BY cidr"#,
        id
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("database error: {e}")))?;

    Ok(SubnetWithChildren { subnet, children })
}

#[server]
pub async fn create_subnet(
    cidr: String,
    name: String,
    description: Option<String>,
    parent_id: Option<Uuid>,
) -> Result<Subnet, ServerFnError> {
    use super::db::pool;
    use ipnetwork::IpNetwork;

    let name = name.trim().to_string();
    if name.is_empty() {
        return Err(ServerFnError::new("subnet name cannot be empty"));
    }
    if name.len() > 255 {
        return Err(ServerFnError::new(
            "subnet name must be 255 characters or less",
        ));
    }

    let pool = pool()?;

    let network: IpNetwork = cidr
        .parse()
        .map_err(|e| ServerFnError::new(format!("invalid CIDR notation: {e}")))?;

    let total_addresses = if network.prefix() >= 31 {
        2_i32.pow(32 - network.prefix() as u32)
    } else {
        2_i32.pow(32 - network.prefix() as u32) - 2
    };

    let id = Uuid::now_v7();
    let subnet: Subnet = sqlx::query_as(
        r#"INSERT INTO subnets (id, cidr, name, description, parent_id, total_addresses)
           VALUES ($1, $2::CIDR, $3, $4, $5, $6)
           RETURNING id, cidr::TEXT as cidr, name, description, parent_id,
           used_count, total_addresses, created_at, updated_at"#,
    )
    .bind(id)
    .bind(&cidr)
    .bind(&name)
    .bind(&description)
    .bind(parent_id)
    .bind(total_addresses)
    .fetch_one(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("failed to create subnet: {e}")))?;

    if let Err(e) = sqlx::query!(
        "INSERT INTO audit_logs (id, entity_type, entity_id, action, new_value)
         VALUES ($1, 'subnet', $2, 'create', $3)",
        Uuid::now_v7(),
        id,
        serde_json::to_value(&subnet).ok()
    )
    .execute(&pool)
    .await
    {
        eprintln!("failed to write audit log for subnet create: {e}");
    }

    Ok(subnet)
}

#[server]
pub async fn update_subnet(
    id: Uuid,
    name: String,
    description: Option<String>,
    parent_id: Option<Uuid>,
) -> Result<Subnet, ServerFnError> {
    use super::db::pool;

    let pool = pool()?;
    let subnet = sqlx::query_as!(
        Subnet,
        r#"UPDATE subnets SET name = $2, description = $3, parent_id = $4
           WHERE id = $1
           RETURNING id, cidr::TEXT as "cidr!", name, description, parent_id,
           used_count, total_addresses, created_at, updated_at"#,
        id,
        name,
        description,
        parent_id
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("database error: {e}")))?
    .ok_or_else(|| ServerFnError::new("Subnet not found"))?;

    Ok(subnet)
}

#[server]
pub async fn delete_subnet(id: Uuid) -> Result<(), ServerFnError> {
    use super::db::pool;

    let pool = pool()?;

    if let Err(e) = sqlx::query!(
        "INSERT INTO audit_logs (id, entity_type, entity_id, action)
         VALUES ($1, 'subnet', $2, 'delete')",
        Uuid::now_v7(),
        id,
    )
    .execute(&pool)
    .await
    {
        eprintln!("failed to write audit log for subnet delete: {e}");
    }

    let result = sqlx::query!("DELETE FROM subnets WHERE id = $1", id)
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("database error: {e}")))?;

    if result.rows_affected() == 0 {
        return Err(ServerFnError::new("Subnet not found"));
    }
    Ok(())
}
