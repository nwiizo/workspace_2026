use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

impl Severity {
    pub const fn weight(self) -> usize {
        match self {
            Self::Low => 1,
            Self::Medium => 2,
            Self::High => 3,
            Self::Critical => 4,
        }
    }

    pub const fn penalty(self) -> f64 {
        match self {
            Self::Low => 1.0,
            Self::Medium => 3.0,
            Self::High => 8.0,
            Self::Critical => 15.0,
        }
    }

    pub const fn id(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Low => f.write_str("Low"),
            Self::Medium => f.write_str("Medium"),
            Self::High => f.write_str("High"),
            Self::Critical => f.write_str("Critical"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Grade {
    A,
    B,
    C,
    D,
    F,
}

impl Grade {
    pub const fn as_char(self) -> char {
        match self {
            Self::A => 'A',
            Self::B => 'B',
            Self::C => 'C',
            Self::D => 'D',
            Self::F => 'F',
        }
    }
}

impl fmt::Display for Grade {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.as_char().to_string())
    }
}

pub fn grade_from_score(score: f64) -> Grade {
    if score >= 90.0 {
        Grade::A
    } else if score >= 80.0 {
        Grade::B
    } else if score >= 70.0 {
        Grade::C
    } else if score >= 60.0 {
        Grade::D
    } else {
        Grade::F
    }
}

pub fn grade_for_severities<I>(severities: I) -> Grade
where
    I: IntoIterator<Item = Severity>,
{
    let mut weighted = 0usize;
    let mut critical = 0usize;
    let mut high = 0usize;
    for severity in severities {
        weighted += severity.weight();
        if severity == Severity::Critical {
            critical += 1;
        } else if severity == Severity::High {
            high += 1;
        }
    }
    if weighted == 0 {
        Grade::A
    } else if critical == 0 && high == 0 && weighted <= 4 {
        Grade::B
    } else if critical == 0 && weighted <= 12 {
        Grade::C
    } else if critical <= 2 && weighted <= 24 {
        Grade::D
    } else {
        Grade::F
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severity_order_and_weight_are_stable() {
        assert!(Severity::Low < Severity::Medium);
        assert!(Severity::Medium < Severity::High);
        assert!(Severity::High < Severity::Critical);
        assert_eq!(Severity::Critical.weight(), 4);
    }

    #[test]
    fn score_and_weighted_grades_match_expected_bands() {
        assert_eq!(grade_from_score(90.0), Grade::A);
        assert_eq!(grade_from_score(80.0), Grade::B);
        assert_eq!(grade_from_score(70.0), Grade::C);
        assert_eq!(grade_from_score(60.0), Grade::D);
        assert_eq!(grade_from_score(59.9), Grade::F);
        assert_eq!(grade_for_severities([]), Grade::A);
        assert_eq!(grade_for_severities([Severity::Low]), Grade::B);
        assert_eq!(
            grade_for_severities([Severity::High, Severity::High, Severity::High]),
            Grade::C
        );
        assert_eq!(
            grade_for_severities([Severity::Critical, Severity::Critical, Severity::Critical]),
            Grade::F
        );
    }
}
