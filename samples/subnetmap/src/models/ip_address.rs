use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum IpStatus {
    Available,
    Assigned,
    Reserved,
    Deprecated,
}

impl IpStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::Assigned => "assigned",
            Self::Reserved => "reserved",
            Self::Deprecated => "deprecated",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "assigned" => Self::Assigned,
            "reserved" => Self::Reserved,
            "deprecated" => Self::Deprecated,
            _ => Self::Available,
        }
    }

    pub fn color_class(&self) -> &'static str {
        match self {
            Self::Available => "bg-green-500/20 text-green-400",
            Self::Assigned => "bg-blue-500/20 text-blue-400",
            Self::Reserved => "bg-yellow-500/20 text-yellow-400",
            Self::Deprecated => "bg-red-500/20 text-red-400",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct IpAddress {
    pub id: Uuid,
    pub address: String,
    pub subnet_id: Uuid,
    pub status: String,
    pub hostname: Option<String>,
    pub assigned_to: Option<String>,
    pub description: Option<String>,
    pub mac_address: Option<String>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpAddressWithMeta {
    pub ip: IpAddress,
    pub subnet_cidr: String,
    pub tags: Vec<super::tag::Tag>,
}
