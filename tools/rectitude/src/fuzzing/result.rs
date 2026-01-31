//! Fuzzing result types
//!
//! Provides structures for tracking and analyzing fuzzing results.

use std::fmt;
use std::time::Duration;

/// Result of a fuzzing session
#[derive(Debug, Clone)]
pub struct FuzzingResult<P: Clone> {
    /// Successful payloads (those that matched success criteria)
    pub successful: Vec<FuzzingHit<P>>,
    /// Failed payloads (those that didn't match success criteria)
    pub failed: Vec<FuzzingMiss<P>>,
    /// Errors encountered during fuzzing
    pub errors: Vec<FuzzingError>,
    /// Statistics about the fuzzing session
    pub stats: FuzzingStats,
}

impl<P: Clone> Default for FuzzingResult<P> {
    fn default() -> Self {
        Self::new()
    }
}

impl<P: Clone> FuzzingResult<P> {
    /// Create a new empty fuzzing result
    pub fn new() -> Self {
        Self {
            successful: Vec::new(),
            failed: Vec::new(),
            errors: Vec::new(),
            stats: FuzzingStats::default(),
        }
    }

    /// Add a successful hit
    pub fn add_hit(&mut self, hit: FuzzingHit<P>) {
        self.successful.push(hit);
        self.stats.successful += 1;
        self.stats.total_attempts += 1;
    }

    /// Add a failed attempt
    pub fn add_miss(&mut self, miss: FuzzingMiss<P>) {
        self.failed.push(miss);
        self.stats.failed += 1;
        self.stats.total_attempts += 1;
    }

    /// Add an error
    pub fn add_error(&mut self, error: FuzzingError) {
        self.errors.push(error);
        self.stats.errors += 1;
        self.stats.total_attempts += 1;
    }

    /// Calculate the success rate
    pub fn success_rate(&self) -> f64 {
        if self.stats.total_attempts == 0 {
            return 0.0;
        }
        self.stats.successful as f64 / self.stats.total_attempts as f64
    }

    /// Check if any payloads were successful
    pub fn has_success(&self) -> bool {
        !self.successful.is_empty()
    }

    /// Check if there were any errors
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Finalize the result with duration
    pub fn finalize(&mut self, duration: Duration) {
        self.stats.duration = duration;
        let secs = duration.as_secs_f64();
        if secs > 0.0 {
            self.stats.requests_per_second = self.stats.total_attempts as f64 / secs;
        }
    }

    /// Generate a text report of the fuzzing results
    pub fn to_report(&self) -> String {
        let mut report = String::new();

        report.push_str("=== Fuzzing Report ===\n\n");

        // Stats
        report.push_str(&format!("Total Attempts: {}\n", self.stats.total_attempts));
        report.push_str(&format!("Successful: {}\n", self.stats.successful));
        report.push_str(&format!("Failed: {}\n", self.stats.failed));
        report.push_str(&format!("Errors: {}\n", self.stats.errors));
        report.push_str(&format!(
            "Success Rate: {:.2}%\n",
            self.success_rate() * 100.0
        ));
        report.push_str(&format!(
            "Duration: {:.2}s\n",
            self.stats.duration.as_secs_f64()
        ));
        report.push_str(&format!(
            "Requests/sec: {:.2}\n",
            self.stats.requests_per_second
        ));
        report.push('\n');

        // Successful payloads
        if !self.successful.is_empty() {
            report.push_str("=== Successful Payloads ===\n");
            for (i, hit) in self.successful.iter().enumerate() {
                report.push_str(&format!(
                    "{}. Status: {}, Duration: {:?}\n",
                    i + 1,
                    hit.response_status,
                    hit.duration
                ));
                if !hit.response_snippet.is_empty() {
                    report.push_str(&format!("   Response: {}\n", hit.response_snippet));
                }
            }
            report.push('\n');
        }

        // Errors
        if !self.errors.is_empty() {
            report.push_str("=== Errors ===\n");
            for (i, error) in self.errors.iter().enumerate() {
                report.push_str(&format!("{}. {}\n", i + 1, error.message));
            }
            report.push('\n');
        }

        report
    }
}

impl<P: Clone + fmt::Display> FuzzingResult<P> {
    /// Generate a detailed report including payload content
    pub fn to_detailed_report(&self) -> String {
        let mut report = self.to_report();

        if !self.successful.is_empty() {
            report.push_str("=== Successful Payload Details ===\n");
            for (i, hit) in self.successful.iter().enumerate() {
                report.push_str(&format!("{}. Payload: {}\n", i + 1, hit.payload));
                report.push_str(&format!(
                    "   Status: {}, Duration: {:?}\n",
                    hit.response_status, hit.duration
                ));
            }
            report.push('\n');
        }

        report
    }
}

/// A successful fuzzing hit
#[derive(Debug, Clone)]
pub struct FuzzingHit<P: Clone> {
    /// The payload that succeeded
    pub payload: P,
    /// HTTP response status code
    pub response_status: u16,
    /// A snippet of the response body
    pub response_snippet: String,
    /// Time taken for this request
    pub duration: Duration,
    /// Additional metadata
    pub metadata: std::collections::HashMap<String, String>,
}

impl<P: Clone> FuzzingHit<P> {
    /// Create a new fuzzing hit
    pub fn new(payload: P, response_status: u16, duration: Duration) -> Self {
        Self {
            payload,
            response_status,
            response_snippet: String::new(),
            duration,
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Set the response snippet
    pub fn with_snippet(mut self, snippet: impl Into<String>) -> Self {
        self.response_snippet = snippet.into();
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// A failed fuzzing attempt
#[derive(Debug, Clone)]
pub struct FuzzingMiss<P: Clone> {
    /// The payload that failed
    pub payload: P,
    /// HTTP response status code
    pub response_status: u16,
    /// Time taken for this request
    pub duration: Duration,
    /// Reason for failure (if known)
    pub reason: Option<String>,
}

impl<P: Clone> FuzzingMiss<P> {
    /// Create a new fuzzing miss
    pub fn new(payload: P, response_status: u16, duration: Duration) -> Self {
        Self {
            payload,
            response_status,
            duration,
            reason: None,
        }
    }

    /// Set the reason for failure
    pub fn with_reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }
}

/// An error encountered during fuzzing
#[derive(Debug, Clone)]
pub struct FuzzingError {
    /// Error message
    pub message: String,
    /// The payload that caused the error (if any)
    pub payload: Option<String>,
    /// Whether this error is recoverable
    pub recoverable: bool,
}

impl FuzzingError {
    /// Create a new fuzzing error
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            payload: None,
            recoverable: true,
        }
    }

    /// Create a non-recoverable error
    pub fn fatal(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            payload: None,
            recoverable: false,
        }
    }

    /// Set the payload that caused the error
    pub fn with_payload(mut self, payload: impl Into<String>) -> Self {
        self.payload = Some(payload.into());
        self
    }
}

impl fmt::Display for FuzzingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ref payload) = self.payload {
            write!(f, "{} (payload: {})", self.message, payload)
        } else {
            write!(f, "{}", self.message)
        }
    }
}

/// Statistics about a fuzzing session
#[derive(Debug, Clone, Default)]
pub struct FuzzingStats {
    /// Total number of attempts made
    pub total_attempts: usize,
    /// Number of successful attempts
    pub successful: usize,
    /// Number of failed attempts
    pub failed: usize,
    /// Number of errors encountered
    pub errors: usize,
    /// Total duration of the fuzzing session
    pub duration: Duration,
    /// Requests per second
    pub requests_per_second: f64,
}

impl FuzzingStats {
    /// Create new empty stats
    pub fn new() -> Self {
        Self::default()
    }

    /// Calculate success rate
    pub fn success_rate(&self) -> f64 {
        if self.total_attempts == 0 {
            return 0.0;
        }
        self.successful as f64 / self.total_attempts as f64
    }

    /// Calculate error rate
    pub fn error_rate(&self) -> f64 {
        if self.total_attempts == 0 {
            return 0.0;
        }
        self.errors as f64 / self.total_attempts as f64
    }
}

impl fmt::Display for FuzzingStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Attempts: {} | Success: {} ({:.1}%) | Failed: {} | Errors: {} | Duration: {:.2}s | {:.1} req/s",
            self.total_attempts,
            self.successful,
            self.success_rate() * 100.0,
            self.failed,
            self.errors,
            self.duration.as_secs_f64(),
            self.requests_per_second
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fuzzing_result_new() {
        let result: FuzzingResult<String> = FuzzingResult::new();
        assert!(result.successful.is_empty());
        assert!(result.failed.is_empty());
        assert!(result.errors.is_empty());
        assert_eq!(result.stats.total_attempts, 0);
    }

    #[test]
    fn test_add_hit() {
        let mut result: FuzzingResult<String> = FuzzingResult::new();
        let hit = FuzzingHit::new("payload".to_string(), 200, Duration::from_millis(100));
        result.add_hit(hit);

        assert_eq!(result.successful.len(), 1);
        assert_eq!(result.stats.successful, 1);
        assert_eq!(result.stats.total_attempts, 1);
    }

    #[test]
    fn test_add_miss() {
        let mut result: FuzzingResult<String> = FuzzingResult::new();
        let miss = FuzzingMiss::new("payload".to_string(), 404, Duration::from_millis(50));
        result.add_miss(miss);

        assert_eq!(result.failed.len(), 1);
        assert_eq!(result.stats.failed, 1);
        assert_eq!(result.stats.total_attempts, 1);
    }

    #[test]
    fn test_success_rate() {
        let mut result: FuzzingResult<String> = FuzzingResult::new();

        // Add 2 hits and 3 misses
        for _ in 0..2 {
            result.add_hit(FuzzingHit::new(
                "hit".to_string(),
                200,
                Duration::from_millis(100),
            ));
        }
        for _ in 0..3 {
            result.add_miss(FuzzingMiss::new(
                "miss".to_string(),
                404,
                Duration::from_millis(50),
            ));
        }

        assert!((result.success_rate() - 0.4).abs() < 0.001);
    }

    #[test]
    fn test_finalize() {
        let mut result: FuzzingResult<String> = FuzzingResult::new();
        for _ in 0..10 {
            result.add_hit(FuzzingHit::new(
                "payload".to_string(),
                200,
                Duration::from_millis(100),
            ));
        }

        result.finalize(Duration::from_secs(2));

        assert_eq!(result.stats.duration, Duration::from_secs(2));
        assert!((result.stats.requests_per_second - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_to_report() {
        let mut result: FuzzingResult<String> = FuzzingResult::new();
        result.add_hit(FuzzingHit::new(
            "payload".to_string(),
            200,
            Duration::from_millis(100),
        ));
        result.finalize(Duration::from_secs(1));

        let report = result.to_report();
        assert!(report.contains("Fuzzing Report"));
        assert!(report.contains("Total Attempts: 1"));
        assert!(report.contains("Successful: 1"));
    }

    #[test]
    fn test_fuzzing_error_display() {
        let error = FuzzingError::new("Connection refused").with_payload("<script>");
        let display = error.to_string();
        assert!(display.contains("Connection refused"));
        assert!(display.contains("<script>"));
    }

    #[test]
    fn test_stats_display() {
        let stats = FuzzingStats {
            total_attempts: 100,
            successful: 25,
            failed: 70,
            errors: 5,
            duration: Duration::from_secs(10),
            requests_per_second: 10.0,
        };

        let display = stats.to_string();
        assert!(display.contains("100"));
        assert!(display.contains("25"));
        assert!(display.contains("25.0%"));
    }
}
