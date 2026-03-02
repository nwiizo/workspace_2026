use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vlan {
    pub id: Uuid,
    pub vlan_id: i32,
    pub name: String,
    pub description: Option<String>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VlanWithSubnets {
    pub vlan: Vlan,
    pub subnets: Vec<super::subnet::Subnet>,
}
