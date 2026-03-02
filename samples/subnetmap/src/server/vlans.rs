use leptos::prelude::*;
use uuid::Uuid;

#[cfg(feature = "ssr")]
use crate::models::subnet::Subnet;
use crate::models::vlan::{Vlan, VlanWithSubnets};

#[server]
pub async fn list_vlans() -> Result<Vec<Vlan>, ServerFnError> {
    use super::db::pool;

    let pool = pool()?;
    let rows = sqlx::query_as!(
        Vlan,
        "SELECT id, vlan_id, name, description, created_at, updated_at
         FROM vlans ORDER BY vlan_id"
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("database error: {e}")))?;

    Ok(rows)
}

#[server]
pub async fn get_vlan(id: Uuid) -> Result<VlanWithSubnets, ServerFnError> {
    use super::db::pool;

    let pool = pool()?;
    let vlan = sqlx::query_as!(
        Vlan,
        "SELECT id, vlan_id, name, description, created_at, updated_at
         FROM vlans WHERE id = $1",
        id
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("database error: {e}")))?
    .ok_or_else(|| ServerFnError::new("VLAN not found"))?;

    let subnets = sqlx::query_as!(
        Subnet,
        r#"SELECT s.id, s.cidr::TEXT as "cidr!", s.name, s.description, s.parent_id,
           s.used_count, s.total_addresses, s.created_at, s.updated_at
           FROM subnets s
           JOIN vlan_subnet_links vsl ON s.id = vsl.subnet_id
           WHERE vsl.vlan_id = $1
           ORDER BY s.cidr"#,
        id
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("database error: {e}")))?;

    Ok(VlanWithSubnets { vlan, subnets })
}

#[server]
pub async fn create_vlan(
    vlan_id: i32,
    name: String,
    description: Option<String>,
) -> Result<Vlan, ServerFnError> {
    use super::db::pool;

    let name = name.trim().to_string();
    if name.is_empty() {
        return Err(ServerFnError::new("VLAN name cannot be empty"));
    }

    if !(1..=4094).contains(&vlan_id) {
        return Err(ServerFnError::new("VLAN ID must be between 1 and 4094"));
    }

    let pool = pool()?;

    let id = Uuid::now_v7();
    let vlan = sqlx::query_as!(
        Vlan,
        "INSERT INTO vlans (id, vlan_id, name, description)
         VALUES ($1, $2, $3, $4)
         RETURNING id, vlan_id, name, description, created_at, updated_at",
        id,
        vlan_id,
        name,
        description
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("database error: {e}")))?;

    if let Err(e) = sqlx::query!(
        "INSERT INTO audit_logs (id, entity_type, entity_id, action, new_value)
         VALUES ($1, 'vlan', $2, 'create', $3)",
        Uuid::now_v7(),
        id,
        serde_json::to_value(&vlan).ok()
    )
    .execute(&pool)
    .await
    {
        eprintln!("failed to write audit log for VLAN create: {e}");
    }

    Ok(vlan)
}

#[server]
pub async fn delete_vlan(id: Uuid) -> Result<(), ServerFnError> {
    use super::db::pool;

    let pool = pool()?;

    let result = sqlx::query!("DELETE FROM vlans WHERE id = $1", id)
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("database error: {e}")))?;

    if result.rows_affected() == 0 {
        return Err(ServerFnError::new("VLAN not found"));
    }
    Ok(())
}

#[server]
pub async fn link_vlan_subnet(vlan_id: Uuid, subnet_id: Uuid) -> Result<(), ServerFnError> {
    use super::db::pool;

    let pool = pool()?;
    sqlx::query!(
        "INSERT INTO vlan_subnet_links (vlan_id, subnet_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        vlan_id,
        subnet_id
    )
    .execute(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("database error: {e}")))?;

    Ok(())
}

#[server]
pub async fn unlink_vlan_subnet(vlan_id: Uuid, subnet_id: Uuid) -> Result<(), ServerFnError> {
    use super::db::pool;

    let pool = pool()?;
    sqlx::query!(
        "DELETE FROM vlan_subnet_links WHERE vlan_id = $1 AND subnet_id = $2",
        vlan_id,
        subnet_id
    )
    .execute(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("database error: {e}")))?;

    Ok(())
}
