use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DnsRecordType {
    A,
    AAAA,
    PTR,
}

impl DnsRecordType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::A => "A",
            Self::AAAA => "AAAA",
            Self::PTR => "PTR",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "AAAA" => Self::AAAA,
            "PTR" => Self::PTR,
            _ => Self::A,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct DnsRecord {
    pub id: Uuid,
    pub record_type: String,
    pub hostname: String,
    pub ip_address_id: Uuid,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}
