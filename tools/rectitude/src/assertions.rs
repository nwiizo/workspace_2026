//! Rich assertion system with fluent API
//!
//! Provides `expect()` builder pattern for expressive assertions.
//!
//! # Example
//! ```ignore
//! use rectitude::assertions::expect;
//!
//! let resp = client.get("/api/users").send().await?;
//!
//! expect(&resp)
//!     .to_be_ok()?
//!     .to_have_status(200)?
//!     .to_contain("users")?
//!     .to_have_header("content-type", Some("application/json"))?;
//! ```

use crate::client::SecurityResponse;
use crate::error::{Error, Result};

/// Create an expectation builder for a value
pub fn expect<T>(value: T) -> Expect<T> {
    Expect::new(value)
}

/// Expectation builder for fluent assertions
#[derive(Debug)]
pub struct Expect<T> {
    value: T,
    negated: bool,
}

impl<T> Expect<T> {
    /// Create a new expectation
    pub fn new(value: T) -> Self {
        Self {
            value,
            negated: false,
        }
    }

    /// Get reference to the inner value
    pub fn value(&self) -> &T {
        &self.value
    }

    /// Consume and return the inner value
    pub fn into_inner(self) -> T {
        self.value
    }

    /// Negate the next assertion (NOT)
    #[allow(clippy::should_implement_trait)]
    pub fn not(mut self) -> Self {
        self.negated = !self.negated;
        self
    }

    /// Reset negation
    fn reset_negated(&mut self) {
        self.negated = false;
    }

    /// Check if the assertion should pass based on negation
    fn check(&self, condition: bool) -> bool {
        if self.negated { !condition } else { condition }
    }
}

// ============ SecurityResponse Assertions ============

impl Expect<&SecurityResponse> {
    /// Assert response is successful (2xx)
    pub fn to_be_ok(&mut self) -> Result<&mut Self> {
        let is_ok = self.value.is_success();
        if !self.check(is_ok) {
            let msg = if self.negated {
                format!("Expected non-success status, got {}", self.value.status)
            } else {
                format!("Expected success status, got {}", self.value.status)
            };
            self.reset_negated();
            return Err(Error::assertion_failed(msg));
        }
        self.reset_negated();
        Ok(self)
    }

    /// Assert response has specific status code
    pub fn to_have_status(&mut self, status: u16) -> Result<&mut Self> {
        let actual = self.value.status.as_u16();
        if !self.check(actual == status) {
            let msg = if self.negated {
                format!("Expected status to not be {}, but it was", status)
            } else {
                format!("Expected status {}, got {}", status, actual)
            };
            self.reset_negated();
            return Err(Error::assertion_failed(msg));
        }
        self.reset_negated();
        Ok(self)
    }

    /// Assert response body contains text
    pub fn to_contain(&mut self, text: &str) -> Result<&mut Self> {
        let contains = self.value.contains(text);
        if !self.check(contains) {
            let msg = if self.negated {
                format!("Expected response to not contain '{}'", text)
            } else {
                format!("Expected response to contain '{}'", text)
            };
            self.reset_negated();
            return Err(Error::assertion_failed(msg));
        }
        self.reset_negated();
        Ok(self)
    }

    /// Assert response has header (optionally with specific value)
    pub fn to_have_header(&mut self, name: &str, value: Option<&str>) -> Result<&mut Self> {
        let header = self.value.header(name);

        let matches = match (header, value) {
            (None, _) => false,
            (Some(_), None) => true,
            (Some(actual), Some(expected)) => actual == expected,
        };

        if !self.check(matches) {
            let msg = if self.negated {
                match value {
                    Some(v) => format!(
                        "Expected header '{}' to not have value '{}', but it did",
                        name, v
                    ),
                    None => format!("Expected header '{}' to not exist, but it did", name),
                }
            } else {
                match (header, value) {
                    (None, _) => format!("Expected header '{}' to exist", name),
                    (Some(actual), Some(expected)) => format!(
                        "Expected header '{}' to be '{}', got '{}'",
                        name, expected, actual
                    ),
                    _ => unreachable!(),
                }
            };
            self.reset_negated();
            return Err(Error::assertion_failed(msg));
        }
        self.reset_negated();
        Ok(self)
    }

    /// Assert JSON path equals expected value
    pub fn to_have_json(&mut self, path: &str, expected: &serde_json::Value) -> Result<&mut Self> {
        let actual = self.value.json_path(path)?;
        if !self.check(&actual == expected) {
            let msg = if self.negated {
                format!("Expected JSON path '{}' to not equal {:?}", path, expected)
            } else {
                format!(
                    "Expected JSON path '{}' to equal {:?}, got {:?}",
                    path, expected, actual
                )
            };
            self.reset_negated();
            return Err(Error::assertion_failed(msg));
        }
        self.reset_negated();
        Ok(self)
    }

    /// Assert response body matches regex
    pub fn to_match(&mut self, pattern: &str) -> Result<&mut Self> {
        let re = regex::Regex::new(pattern).map_err(|e| Error::InvalidInput(e.to_string()))?;
        let matches = re.is_match(self.value.text());
        if !self.check(matches) {
            let msg = if self.negated {
                format!("Expected response to not match pattern '{}'", pattern)
            } else {
                format!("Expected response to match pattern '{}'", pattern)
            };
            self.reset_negated();
            return Err(Error::assertion_failed(msg));
        }
        self.reset_negated();
        Ok(self)
    }

    /// Assert response body length
    pub fn to_have_length(&mut self, len: usize) -> Result<&mut Self> {
        let actual = self.value.body_len();
        if !self.check(actual == len) {
            let msg = if self.negated {
                format!("Expected body length to not be {}", len)
            } else {
                format!("Expected body length {}, got {}", len, actual)
            };
            self.reset_negated();
            return Err(Error::assertion_failed(msg));
        }
        self.reset_negated();
        Ok(self)
    }

    /// Assert body length is greater than
    pub fn to_have_length_gt(&mut self, len: usize) -> Result<&mut Self> {
        let actual = self.value.body_len();
        if !self.check(actual > len) {
            let msg = if self.negated {
                format!("Expected body length to not be greater than {}", len)
            } else {
                format!("Expected body length > {}, got {}", len, actual)
            };
            self.reset_negated();
            return Err(Error::assertion_failed(msg));
        }
        self.reset_negated();
        Ok(self)
    }

    /// Assert body length is less than
    pub fn to_have_length_lt(&mut self, len: usize) -> Result<&mut Self> {
        let actual = self.value.body_len();
        if !self.check(actual < len) {
            let msg = if self.negated {
                format!("Expected body length to not be less than {}", len)
            } else {
                format!("Expected body length < {}, got {}", len, actual)
            };
            self.reset_negated();
            return Err(Error::assertion_failed(msg));
        }
        self.reset_negated();
        Ok(self)
    }

    /// Assert response is a redirect (3xx)
    pub fn to_be_redirect(&mut self) -> Result<&mut Self> {
        let is_redirect = self.value.status.is_redirection();
        if !self.check(is_redirect) {
            let msg = if self.negated {
                format!("Expected non-redirect status, got {}", self.value.status)
            } else {
                format!("Expected redirect status (3xx), got {}", self.value.status)
            };
            self.reset_negated();
            return Err(Error::assertion_failed(msg));
        }
        self.reset_negated();
        Ok(self)
    }

    /// Assert response is a client error (4xx)
    pub fn to_be_client_error(&mut self) -> Result<&mut Self> {
        let is_client_error = self.value.status.is_client_error();
        if !self.check(is_client_error) {
            let msg = if self.negated {
                format!(
                    "Expected non-client-error status, got {}",
                    self.value.status
                )
            } else {
                format!(
                    "Expected client error status (4xx), got {}",
                    self.value.status
                )
            };
            self.reset_negated();
            return Err(Error::assertion_failed(msg));
        }
        self.reset_negated();
        Ok(self)
    }

    /// Assert response is a server error (5xx)
    pub fn to_be_server_error(&mut self) -> Result<&mut Self> {
        let is_server_error = self.value.status.is_server_error();
        if !self.check(is_server_error) {
            let msg = if self.negated {
                format!(
                    "Expected non-server-error status, got {}",
                    self.value.status
                )
            } else {
                format!(
                    "Expected server error status (5xx), got {}",
                    self.value.status
                )
            };
            self.reset_negated();
            return Err(Error::assertion_failed(msg));
        }
        self.reset_negated();
        Ok(self)
    }
}

// ============ String Assertions ============

impl Expect<&str> {
    /// Assert string equals expected
    pub fn to_equal(&mut self, expected: &str) -> Result<&mut Self> {
        if !self.check(self.value == expected) {
            let msg = if self.negated {
                format!("Expected string to not equal '{}'", expected)
            } else {
                format!("Expected '{}', got '{}'", expected, self.value)
            };
            self.reset_negated();
            return Err(Error::assertion_failed(msg));
        }
        self.reset_negated();
        Ok(self)
    }

    /// Assert string contains substring
    pub fn to_contain(&mut self, substring: &str) -> Result<&mut Self> {
        if !self.check(self.value.contains(substring)) {
            let msg = if self.negated {
                format!("Expected string to not contain '{}'", substring)
            } else {
                format!("Expected string to contain '{}'", substring)
            };
            self.reset_negated();
            return Err(Error::assertion_failed(msg));
        }
        self.reset_negated();
        Ok(self)
    }

    /// Assert string starts with prefix
    pub fn to_start_with(&mut self, prefix: &str) -> Result<&mut Self> {
        if !self.check(self.value.starts_with(prefix)) {
            let msg = if self.negated {
                format!("Expected string to not start with '{}'", prefix)
            } else {
                format!("Expected string to start with '{}'", prefix)
            };
            self.reset_negated();
            return Err(Error::assertion_failed(msg));
        }
        self.reset_negated();
        Ok(self)
    }

    /// Assert string ends with suffix
    pub fn to_end_with(&mut self, suffix: &str) -> Result<&mut Self> {
        if !self.check(self.value.ends_with(suffix)) {
            let msg = if self.negated {
                format!("Expected string to not end with '{}'", suffix)
            } else {
                format!("Expected string to end with '{}'", suffix)
            };
            self.reset_negated();
            return Err(Error::assertion_failed(msg));
        }
        self.reset_negated();
        Ok(self)
    }

    /// Assert string is empty
    pub fn to_be_empty(&mut self) -> Result<&mut Self> {
        if !self.check(self.value.is_empty()) {
            let msg = if self.negated {
                "Expected string to not be empty".to_string()
            } else {
                format!("Expected string to be empty, got '{}'", self.value)
            };
            self.reset_negated();
            return Err(Error::assertion_failed(msg));
        }
        self.reset_negated();
        Ok(self)
    }

    /// Assert string matches regex
    pub fn to_match(&mut self, pattern: &str) -> Result<&mut Self> {
        let re = regex::Regex::new(pattern).map_err(|e| Error::InvalidInput(e.to_string()))?;
        if !self.check(re.is_match(self.value)) {
            let msg = if self.negated {
                format!("Expected string to not match pattern '{}'", pattern)
            } else {
                format!("Expected string to match pattern '{}'", pattern)
            };
            self.reset_negated();
            return Err(Error::assertion_failed(msg));
        }
        self.reset_negated();
        Ok(self)
    }
}

// ============ Numeric Assertions ============

macro_rules! impl_numeric_assertions {
    ($($t:ty),*) => {
        $(
            impl Expect<$t> {
                /// Assert number equals expected
                pub fn to_equal(&mut self, expected: $t) -> Result<&mut Self> {
                    if !self.check(self.value == expected) {
                        let msg = if self.negated {
                            format!("Expected value to not equal {}", expected)
                        } else {
                            format!("Expected {}, got {}", expected, self.value)
                        };
                        self.reset_negated();
                        return Err(Error::assertion_failed(msg));
                    }
                    self.reset_negated();
                    Ok(self)
                }

                /// Assert number is greater than
                pub fn to_be_gt(&mut self, expected: $t) -> Result<&mut Self> {
                    if !self.check(self.value > expected) {
                        let msg = if self.negated {
                            format!("Expected value to not be greater than {}", expected)
                        } else {
                            format!("Expected value > {}, got {}", expected, self.value)
                        };
                        self.reset_negated();
                        return Err(Error::assertion_failed(msg));
                    }
                    self.reset_negated();
                    Ok(self)
                }

                /// Assert number is less than
                pub fn to_be_lt(&mut self, expected: $t) -> Result<&mut Self> {
                    if !self.check(self.value < expected) {
                        let msg = if self.negated {
                            format!("Expected value to not be less than {}", expected)
                        } else {
                            format!("Expected value < {}, got {}", expected, self.value)
                        };
                        self.reset_negated();
                        return Err(Error::assertion_failed(msg));
                    }
                    self.reset_negated();
                    Ok(self)
                }

                /// Assert number is greater than or equal
                pub fn to_be_gte(&mut self, expected: $t) -> Result<&mut Self> {
                    if !self.check(self.value >= expected) {
                        let msg = if self.negated {
                            format!("Expected value to not be >= {}", expected)
                        } else {
                            format!("Expected value >= {}, got {}", expected, self.value)
                        };
                        self.reset_negated();
                        return Err(Error::assertion_failed(msg));
                    }
                    self.reset_negated();
                    Ok(self)
                }

                /// Assert number is less than or equal
                pub fn to_be_lte(&mut self, expected: $t) -> Result<&mut Self> {
                    if !self.check(self.value <= expected) {
                        let msg = if self.negated {
                            format!("Expected value to not be <= {}", expected)
                        } else {
                            format!("Expected value <= {}, got {}", expected, self.value)
                        };
                        self.reset_negated();
                        return Err(Error::assertion_failed(msg));
                    }
                    self.reset_negated();
                    Ok(self)
                }

                /// Assert number is in range (inclusive)
                pub fn to_be_in_range(&mut self, min: $t, max: $t) -> Result<&mut Self> {
                    let in_range = self.value >= min && self.value <= max;
                    if !self.check(in_range) {
                        let msg = if self.negated {
                            format!("Expected value to not be in range [{}, {}]", min, max)
                        } else {
                            format!("Expected value in range [{}, {}], got {}", min, max, self.value)
                        };
                        self.reset_negated();
                        return Err(Error::assertion_failed(msg));
                    }
                    self.reset_negated();
                    Ok(self)
                }
            }
        )*
    };
}

impl_numeric_assertions!(i32, i64, u32, u64, usize, f32, f64);

// ============ Boolean Assertions ============

impl Expect<bool> {
    /// Assert boolean is true
    pub fn to_be_true(&mut self) -> Result<&mut Self> {
        if !self.check(self.value) {
            let msg = if self.negated {
                "Expected value to not be true".to_string()
            } else {
                "Expected true, got false".to_string()
            };
            self.reset_negated();
            return Err(Error::assertion_failed(msg));
        }
        self.reset_negated();
        Ok(self)
    }

    /// Assert boolean is false
    pub fn to_be_false(&mut self) -> Result<&mut Self> {
        if !self.check(!self.value) {
            let msg = if self.negated {
                "Expected value to not be false".to_string()
            } else {
                "Expected false, got true".to_string()
            };
            self.reset_negated();
            return Err(Error::assertion_failed(msg));
        }
        self.reset_negated();
        Ok(self)
    }
}

// ============ Option Assertions ============

impl<T: std::fmt::Debug> Expect<Option<T>> {
    /// Assert option is Some
    pub fn to_be_some(&mut self) -> Result<&mut Self> {
        if !self.check(self.value.is_some()) {
            let msg = if self.negated {
                "Expected None, got Some".to_string()
            } else {
                "Expected Some, got None".to_string()
            };
            self.reset_negated();
            return Err(Error::assertion_failed(msg));
        }
        self.reset_negated();
        Ok(self)
    }

    /// Assert option is None
    pub fn to_be_none(&mut self) -> Result<&mut Self> {
        if !self.check(self.value.is_none()) {
            let msg = if self.negated {
                "Expected Some, got None".to_string()
            } else {
                format!("Expected None, got {:?}", self.value)
            };
            self.reset_negated();
            return Err(Error::assertion_failed(msg));
        }
        self.reset_negated();
        Ok(self)
    }
}

// ============ Vec Assertions ============

impl<T: std::fmt::Debug> Expect<&Vec<T>> {
    /// Assert vec is empty
    pub fn to_be_empty(&mut self) -> Result<&mut Self> {
        if !self.check(self.value.is_empty()) {
            let msg = if self.negated {
                "Expected vec to not be empty".to_string()
            } else {
                format!("Expected vec to be empty, got {} items", self.value.len())
            };
            self.reset_negated();
            return Err(Error::assertion_failed(msg));
        }
        self.reset_negated();
        Ok(self)
    }

    /// Assert vec has length
    pub fn to_have_length(&mut self, len: usize) -> Result<&mut Self> {
        if !self.check(self.value.len() == len) {
            let msg = if self.negated {
                format!("Expected vec to not have length {}", len)
            } else {
                format!("Expected vec length {}, got {}", len, self.value.len())
            };
            self.reset_negated();
            return Err(Error::assertion_failed(msg));
        }
        self.reset_negated();
        Ok(self)
    }
}

impl<T: std::fmt::Debug + PartialEq> Expect<&Vec<T>> {
    /// Assert vec contains item
    pub fn to_contain(&mut self, item: &T) -> Result<&mut Self> {
        if !self.check(self.value.contains(item)) {
            let msg = if self.negated {
                format!("Expected vec to not contain {:?}", item)
            } else {
                format!("Expected vec to contain {:?}", item)
            };
            self.reset_negated();
            return Err(Error::assertion_failed(msg));
        }
        self.reset_negated();
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::StatusCode;
    use std::collections::HashMap;

    fn mock_response(status: u16, body: &str) -> SecurityResponse {
        SecurityResponse {
            status: StatusCode::from_u16(status).unwrap(),
            headers: HashMap::new(),
            body: body.as_bytes().to_vec(),
            text: Some(body.to_string()),
        }
    }

    fn mock_response_with_headers(
        status: u16,
        body: &str,
        headers: Vec<(&str, &str)>,
    ) -> SecurityResponse {
        let mut resp = mock_response(status, body);
        for (k, v) in headers {
            resp.headers.insert(k.to_string(), v.to_string());
        }
        resp
    }

    #[test]
    fn test_response_to_be_ok() {
        let resp = mock_response(200, "OK");
        expect(&resp).to_be_ok().unwrap();

        let resp = mock_response(404, "Not Found");
        assert!(expect(&resp).to_be_ok().is_err());
    }

    #[test]
    fn test_response_not() {
        let resp = mock_response(404, "Not Found");
        expect(&resp).not().to_be_ok().unwrap();
    }

    #[test]
    fn test_response_to_have_status() {
        let resp = mock_response(201, "Created");
        expect(&resp).to_have_status(201).unwrap();
        assert!(expect(&resp).to_have_status(200).is_err());
    }

    #[test]
    fn test_response_to_contain() {
        let resp = mock_response(200, "Hello World");
        expect(&resp).to_contain("World").unwrap();
        assert!(expect(&resp).to_contain("Universe").is_err());
    }

    #[test]
    fn test_response_to_have_header() {
        let resp =
            mock_response_with_headers(200, "OK", vec![("content-type", "application/json")]);

        expect(&resp).to_have_header("content-type", None).unwrap();
        expect(&resp)
            .to_have_header("content-type", Some("application/json"))
            .unwrap();
        assert!(expect(&resp).to_have_header("x-custom", None).is_err());
    }

    #[test]
    fn test_string_assertions() {
        expect("hello world")
            .to_contain("world")
            .unwrap()
            .to_start_with("hello")
            .unwrap()
            .to_end_with("world")
            .unwrap();

        assert!(expect("hello").to_be_empty().is_err());
        expect("").to_be_empty().unwrap();
    }

    #[test]
    fn test_numeric_assertions() {
        expect(42i32)
            .to_equal(42)
            .unwrap()
            .to_be_gt(41)
            .unwrap()
            .to_be_lt(43)
            .unwrap()
            .to_be_in_range(40, 45)
            .unwrap();
    }

    #[test]
    fn test_bool_assertions() {
        expect(true).to_be_true().unwrap();
        expect(false).to_be_false().unwrap();
        assert!(expect(true).to_be_false().is_err());
    }

    #[test]
    fn test_option_assertions() {
        let some: Option<i32> = Some(42);
        let none: Option<i32> = None;

        expect(some).to_be_some().unwrap();
        expect(none).to_be_none().unwrap();
        assert!(expect(none).to_be_some().is_err());
    }

    #[test]
    fn test_vec_assertions() {
        let vec = vec![1, 2, 3];
        expect(&vec)
            .to_have_length(3)
            .unwrap()
            .to_contain(&2)
            .unwrap();

        assert!(expect(&vec).to_be_empty().is_err());

        let empty: Vec<i32> = vec![];
        expect(&empty).to_be_empty().unwrap();
    }

    #[test]
    fn test_chained_assertions() {
        let resp = mock_response_with_headers(
            200,
            r#"{"status": "ok"}"#,
            vec![("content-type", "application/json")],
        );

        expect(&resp)
            .to_be_ok()
            .unwrap()
            .to_have_status(200)
            .unwrap()
            .to_have_header("content-type", Some("application/json"))
            .unwrap()
            .to_contain("ok")
            .unwrap();
    }
}
