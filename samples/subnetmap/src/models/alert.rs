use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AlertType {
    HighUtilization,
    DuplicateIp,
    SubnetFull,
}

impl AlertType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::HighUtilization => "high_utilization",
            Self::DuplicateIp => "duplicate_ip",
            Self::SubnetFull => "subnet_full",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "duplicate_ip" => Self::DuplicateIp,
            "subnet_full" => Self::SubnetFull,
            _ => Self::HighUtilization,
        }
    }

    pub fn color_class(&self) -> &'static str {
        match self {
            Self::HighUtilization => "bg-yellow-500/20 text-yellow-400",
            Self::DuplicateIp => "bg-red-500/20 text-red-400",
            Self::SubnetFull => "bg-red-500/20 text-red-400",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct Alert {
    pub id: Uuid,
    pub alert_type: String,
    pub subnet_id: Option<Uuid>,
    pub message: String,
    pub is_acknowledged: bool,
    pub created_at: chrono::NaiveDateTime,
}
