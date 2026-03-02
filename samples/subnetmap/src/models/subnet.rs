use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct Subnet {
    pub id: Uuid,
    pub cidr: String,
    pub name: String,
    pub description: Option<String>,
    pub parent_id: Option<Uuid>,
    pub used_count: i32,
    pub total_addresses: i32,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubnetWithChildren {
    pub subnet: Subnet,
    pub children: Vec<Subnet>,
}

impl Subnet {
    pub fn utilization_percent(&self) -> f64 {
        if self.total_addresses == 0 {
            return 0.0;
        }
        (self.used_count as f64 / self.total_addresses as f64) * 100.0
    }
}
