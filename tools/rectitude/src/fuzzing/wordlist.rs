//! Built-in wordlists for fuzzing
//!
//! Provides common wordlists for various fuzzing scenarios including
//! authentication testing, path discovery, and parameter enumeration.

/// Common usernames for authentication testing
pub fn common_usernames() -> Vec<&'static str> {
    vec![
        // Standard admin accounts
        "admin",
        "administrator",
        "root",
        "superuser",
        "sysadmin",
        // Common defaults
        "user",
        "test",
        "guest",
        "demo",
        "default",
        // Service accounts
        "system",
        "service",
        "backup",
        "operator",
        "support",
        // Role-based
        "manager",
        "moderator",
        "staff",
        "developer",
        "devops",
        // Application specific
        "webmaster",
        "www",
        "ftp",
        "mail",
        "postmaster",
        // Common names
        "john",
        "jane",
        "bob",
        "alice",
        "mike",
        // IT/Security
        "security",
        "audit",
        "monitor",
        "nagios",
        "oracle",
        // Database
        "postgres",
        "mysql",
        "mongodb",
        "redis",
        "elastic",
        // Cloud
        "aws",
        "azure",
        "gcp",
        "cloud",
        "deploy",
        // CI/CD
        "jenkins",
        "gitlab",
        "github",
        "circleci",
        "travis",
        // Anonymous
        "anonymous",
        "anon",
        "nobody",
        "null",
        "void",
    ]
}

/// Common passwords for authentication testing
pub fn common_passwords() -> Vec<&'static str> {
    vec![
        // Top passwords
        "password",
        "123456",
        "12345678",
        "123456789",
        "qwerty",
        "abc123",
        "password1",
        "password123",
        "admin",
        "admin123",
        "letmein",
        "welcome",
        "welcome1",
        // Simple patterns
        "111111",
        "000000",
        "1234567",
        "1234567890",
        "iloveyou",
        "sunshine",
        // Company defaults
        "changeme",
        "password!",
        "P@ssw0rd",
        "P@ssword1",
        "Password1",
        "Password1!",
        // Keyboard patterns
        "qwerty123",
        "qwertyuiop",
        "asdfghjkl",
        "zxcvbnm",
        "1qaz2wsx",
        // Seasons/years
        "summer2024",
        "winter2024",
        "spring2024",
        "fall2024",
        "password2024",
        // Simple words
        "football",
        "baseball",
        "dragon",
        "master",
        "monkey",
        "shadow",
        "michael",
        "jennifer",
        "trustno1",
        // Empty/null
        "",
        "null",
        "none",
        // Default service passwords
        "root",
        "toor",
        "admin",
        "guest",
        "test",
        "demo",
        "default",
        "service",
    ]
}

/// Common file paths for path traversal testing
pub fn common_paths() -> Vec<&'static str> {
    vec![
        // Unix system files
        "/etc/passwd",
        "/etc/shadow",
        "/etc/hosts",
        "/etc/hostname",
        "/etc/resolv.conf",
        "/etc/ssh/sshd_config",
        "/etc/nginx/nginx.conf",
        "/etc/apache2/apache2.conf",
        "/etc/httpd/httpd.conf",
        "/etc/mysql/my.cnf",
        "/etc/postgresql/postgresql.conf",
        // User directories
        "/root/.ssh/id_rsa",
        "/root/.ssh/authorized_keys",
        "/root/.bash_history",
        "/root/.bashrc",
        "/home/user/.ssh/id_rsa",
        // Log files
        "/var/log/auth.log",
        "/var/log/syslog",
        "/var/log/messages",
        "/var/log/apache2/access.log",
        "/var/log/apache2/error.log",
        "/var/log/nginx/access.log",
        "/var/log/nginx/error.log",
        // Process info
        "/proc/self/environ",
        "/proc/self/cmdline",
        "/proc/self/fd/0",
        "/proc/version",
        "/proc/cpuinfo",
        // Application files
        "/var/www/html/index.php",
        "/var/www/html/config.php",
        "/var/www/html/wp-config.php",
        "/var/www/html/.htaccess",
        "/var/www/html/web.config",
        // Windows system files
        "C:\\Windows\\System32\\config\\SAM",
        "C:\\Windows\\System32\\config\\SYSTEM",
        "C:\\Windows\\System32\\drivers\\etc\\hosts",
        "C:\\Windows\\win.ini",
        "C:\\Windows\\php.ini",
        "C:\\inetpub\\wwwroot\\web.config",
        // Application config
        ".env",
        ".env.local",
        ".env.production",
        "config.php",
        "config.json",
        "config.yml",
        "settings.py",
        "application.properties",
        "database.yml",
        "secrets.yml",
    ]
}

/// Common subdomains for enumeration
pub fn common_subdomains() -> Vec<&'static str> {
    vec![
        // Standard
        "www",
        "mail",
        "ftp",
        "smtp",
        "pop",
        "imap",
        "webmail",
        // Development
        "dev",
        "development",
        "test",
        "testing",
        "qa",
        "staging",
        "stage",
        "uat",
        "sandbox",
        "demo",
        "preview",
        "beta",
        "alpha",
        // Administration
        "admin",
        "administrator",
        "manage",
        "management",
        "portal",
        "cp",
        "cpanel",
        "panel",
        "dashboard",
        "console",
        // API
        "api",
        "api-v1",
        "api-v2",
        "rest",
        "graphql",
        "gateway",
        "ws",
        "websocket",
        // Infrastructure
        "ns",
        "ns1",
        "ns2",
        "dns",
        "dns1",
        "dns2",
        "mx",
        "mx1",
        "mx2",
        // Static content
        "static",
        "cdn",
        "assets",
        "images",
        "img",
        "media",
        "files",
        "download",
        "downloads",
        // Security
        "vpn",
        "secure",
        "ssl",
        "sso",
        "auth",
        "login",
        "signin",
        // Monitoring
        "status",
        "health",
        "monitor",
        "monitoring",
        "metrics",
        "grafana",
        "kibana",
        "prometheus",
        // Database
        "db",
        "database",
        "mysql",
        "postgres",
        "mongo",
        "redis",
        "elastic",
        // Cloud
        "cloud",
        "aws",
        "azure",
        "gcp",
        "s3",
        // Internal
        "internal",
        "intranet",
        "corp",
        "corporate",
        "office",
        // Support
        "support",
        "help",
        "helpdesk",
        "docs",
        "documentation",
        "wiki",
        "kb",
        "knowledge",
        // Services
        "git",
        "gitlab",
        "github",
        "jenkins",
        "ci",
        "cd",
        "build",
        "deploy",
        "jira",
        "confluence",
        "slack",
    ]
}

/// Common API endpoints
pub fn common_endpoints() -> Vec<&'static str> {
    vec![
        // Authentication
        "/api/login",
        "/api/logout",
        "/api/auth",
        "/api/authenticate",
        "/api/register",
        "/api/signup",
        "/api/password/reset",
        "/api/password/forgot",
        "/api/token",
        "/api/refresh",
        "/api/oauth",
        "/api/oauth/token",
        "/api/session",
        // User management
        "/api/users",
        "/api/user",
        "/api/me",
        "/api/profile",
        "/api/account",
        "/api/settings",
        "/api/preferences",
        "/api/roles",
        "/api/permissions",
        // CRUD endpoints
        "/api/items",
        "/api/products",
        "/api/orders",
        "/api/posts",
        "/api/comments",
        "/api/files",
        "/api/uploads",
        "/api/images",
        "/api/documents",
        // Admin endpoints
        "/api/admin",
        "/api/admin/users",
        "/api/admin/settings",
        "/api/admin/config",
        "/api/admin/logs",
        "/api/admin/stats",
        "/api/admin/dashboard",
        // System endpoints
        "/api/health",
        "/api/status",
        "/api/ping",
        "/api/version",
        "/api/info",
        "/api/config",
        "/api/env",
        "/api/debug",
        "/api/test",
        // GraphQL
        "/graphql",
        "/graphiql",
        "/playground",
        "/api/graphql",
        // Swagger/OpenAPI
        "/swagger",
        "/swagger.json",
        "/swagger.yaml",
        "/api-docs",
        "/openapi",
        "/openapi.json",
        "/docs",
        "/redoc",
        // Metrics/Monitoring
        "/metrics",
        "/actuator",
        "/actuator/health",
        "/actuator/info",
        "/actuator/metrics",
        "/actuator/env",
        "/.well-known",
        // Legacy
        "/api/v1",
        "/api/v2",
        "/rest",
        "/service",
        "/services",
        "/ws",
        "/websocket",
        "/socket.io",
        // Hidden/Debug
        "/.git",
        "/.git/config",
        "/.env",
        "/backup",
        "/backup.sql",
        "/dump.sql",
        "/phpinfo.php",
        "/info.php",
        "/server-status",
        "/server-info",
    ]
}

/// Common parameter names
pub fn common_params() -> Vec<&'static str> {
    vec![
        // Authentication
        "username",
        "user",
        "email",
        "mail",
        "login",
        "password",
        "pass",
        "passwd",
        "pwd",
        "token",
        "api_key",
        "apikey",
        "api-key",
        "key",
        "secret",
        "auth",
        "access_token",
        "refresh_token",
        "session",
        "sessionid",
        "session_id",
        // Identification
        "id",
        "uid",
        "user_id",
        "userId",
        "account_id",
        "accountId",
        "item_id",
        "itemId",
        "order_id",
        "orderId",
        // Pagination
        "page",
        "p",
        "limit",
        "size",
        "offset",
        "skip",
        "count",
        "per_page",
        "perPage",
        "page_size",
        "pageSize",
        // Search/Filter
        "q",
        "query",
        "search",
        "s",
        "keyword",
        "keywords",
        "filter",
        "sort",
        "order",
        "orderby",
        "order_by",
        "sortby",
        "sort_by",
        "direction",
        "dir",
        // Content
        "name",
        "title",
        "description",
        "desc",
        "content",
        "body",
        "text",
        "message",
        "msg",
        "comment",
        "data",
        "value",
        "values",
        // URLs/Files
        "url",
        "uri",
        "link",
        "href",
        "path",
        "file",
        "filename",
        "filepath",
        "image",
        "img",
        "src",
        "source",
        "target",
        "dest",
        "destination",
        "redirect",
        "redirect_url",
        "redirectUrl",
        "return",
        "return_url",
        "returnUrl",
        "next",
        "callback",
        "callback_url",
        // Actions
        "action",
        "cmd",
        "command",
        "exec",
        "execute",
        "run",
        "do",
        "op",
        "operation",
        "method",
        "function",
        "func",
        // Format
        "format",
        "type",
        "mode",
        "view",
        "output",
        "encoding",
        "charset",
        "lang",
        "language",
        "locale",
        // Debug/Admin
        "debug",
        "test",
        "admin",
        "root",
        "verbose",
        "trace",
        "log",
        "raw",
    ]
}

/// Common file extensions for fuzzing
pub fn common_extensions() -> Vec<&'static str> {
    vec![
        // Web
        ".html", ".htm", ".php", ".php3", ".php4", ".php5", ".phtml", ".asp", ".aspx", ".jsp",
        ".jspx", ".cgi", ".pl", // Config
        ".config", ".conf", ".cfg", ".ini", ".xml", ".json", ".yaml", ".yml", ".toml", ".env",
        // Code
        ".js", ".ts", ".py", ".rb", ".java", ".class", ".jar", ".war", ".ear", // Data
        ".sql", ".db", ".sqlite", ".mdb", ".csv", ".log", ".txt", ".dat", // Backup
        ".bak", ".backup", ".old", ".orig", ".save", ".swp", ".tmp", ".temp", "~", ".copy",
        // Archive
        ".zip", ".tar", ".tar.gz", ".tgz", ".gz", ".rar", ".7z", // Secrets
        ".key", ".pem", ".crt", ".cer", ".p12", ".pfx", ".jks",
    ]
}

/// Common HTTP headers for fuzzing
pub fn common_headers() -> Vec<&'static str> {
    vec![
        // Standard
        "Host",
        "User-Agent",
        "Accept",
        "Accept-Language",
        "Accept-Encoding",
        "Content-Type",
        "Content-Length",
        "Connection",
        "Cookie",
        "Authorization",
        "Referer",
        "Origin",
        // Security
        "X-Forwarded-For",
        "X-Forwarded-Host",
        "X-Forwarded-Proto",
        "X-Real-IP",
        "X-Originating-IP",
        "X-Remote-IP",
        "X-Remote-Addr",
        "X-Client-IP",
        "X-Host",
        "True-Client-IP",
        "Forwarded",
        "Via",
        // Custom
        "X-Custom-IP-Authorization",
        "X-API-Key",
        "X-Auth-Token",
        "X-CSRF-Token",
        "X-Request-ID",
        "X-Correlation-ID",
        // Debug
        "X-Debug",
        "X-Debug-Mode",
        "X-Debug-Token",
        "X-Original-URL",
        "X-Rewrite-URL",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_common_usernames() {
        let usernames = common_usernames();
        assert!(!usernames.is_empty());
        assert!(usernames.contains(&"admin"));
        assert!(usernames.contains(&"root"));
    }

    #[test]
    fn test_common_passwords() {
        let passwords = common_passwords();
        assert!(!passwords.is_empty());
        assert!(passwords.contains(&"password"));
        assert!(passwords.contains(&"123456"));
    }

    #[test]
    fn test_common_paths() {
        let paths = common_paths();
        assert!(!paths.is_empty());
        assert!(paths.contains(&"/etc/passwd"));
    }

    #[test]
    fn test_common_subdomains() {
        let subdomains = common_subdomains();
        assert!(!subdomains.is_empty());
        assert!(subdomains.contains(&"www"));
        assert!(subdomains.contains(&"admin"));
    }

    #[test]
    fn test_common_endpoints() {
        let endpoints = common_endpoints();
        assert!(!endpoints.is_empty());
        assert!(endpoints.contains(&"/api/login"));
    }

    #[test]
    fn test_common_params() {
        let params = common_params();
        assert!(!params.is_empty());
        assert!(params.contains(&"id"));
        assert!(params.contains(&"username"));
        assert!(params.contains(&"password"));
    }

    #[test]
    fn test_common_extensions() {
        let extensions = common_extensions();
        assert!(!extensions.is_empty());
        assert!(extensions.contains(&".php"));
        assert!(extensions.contains(&".bak"));
    }

    #[test]
    fn test_common_headers() {
        let headers = common_headers();
        assert!(!headers.is_empty());
        assert!(headers.contains(&"X-Forwarded-For"));
        assert!(headers.contains(&"Authorization"));
    }
}
