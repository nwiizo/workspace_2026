use std::str::FromStr;

use thiserror::Error;
use time::{Date, Duration, macros::format_description};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Uniform {
    Gi,
    NoGi,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecallRating {
    Forgot,
    Fuzzy,
    Clear,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplicationResult {
    NotTried,
    Attempted,
    Worked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReviewSchedule {
    pub next_review_on: Date,
}

#[derive(Debug, Error, PartialEq, Eq)]
#[error("入力値が許可された選択肢ではありません")]
pub struct InvalidChoice;

#[derive(Debug, Error, PartialEq, Eq)]
#[error("日付が正しくありません")]
pub struct InvalidDate;

impl Uniform {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Gi => "gi",
            Self::NoGi => "no_gi",
        }
    }
}

impl FromStr for Uniform {
    type Err = InvalidChoice;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "gi" => Ok(Self::Gi),
            "no_gi" => Ok(Self::NoGi),
            _ => Err(InvalidChoice),
        }
    }
}

impl RecallRating {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Forgot => "forgot",
            Self::Fuzzy => "fuzzy",
            Self::Clear => "clear",
        }
    }
}

impl FromStr for RecallRating {
    type Err = InvalidChoice;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "forgot" => Ok(Self::Forgot),
            "fuzzy" => Ok(Self::Fuzzy),
            "clear" => Ok(Self::Clear),
            _ => Err(InvalidChoice),
        }
    }
}

impl ApplicationResult {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NotTried => "not_tried",
            Self::Attempted => "attempted",
            Self::Worked => "worked",
        }
    }
}

impl FromStr for ApplicationResult {
    type Err = InvalidChoice;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "not_tried" => Ok(Self::NotTried),
            "attempted" => Ok(Self::Attempted),
            "worked" => Ok(Self::Worked),
            _ => Err(InvalidChoice),
        }
    }
}

/// Parses the calendar-date representation accepted at the HTTP boundary.
///
/// # Errors
///
/// Returns [`InvalidDate`] for malformed or impossible ISO dates.
pub fn parse_iso_date(value: &str) -> Result<Date, InvalidDate> {
    Date::parse(value, format_description!("[year]-[month]-[day]")).map_err(|_| InvalidDate)
}

#[must_use]
pub fn schedule_review(
    reviewed_on: Date,
    recall: RecallRating,
    application: ApplicationResult,
) -> ReviewSchedule {
    let days = match (recall, application) {
        (RecallRating::Forgot, _) => 1,
        (RecallRating::Fuzzy, _) => 3,
        (RecallRating::Clear, ApplicationResult::Worked) => 14,
        (RecallRating::Clear, _) => 7,
    };

    ReviewSchedule {
        next_review_on: reviewed_on.saturating_add(Duration::days(days)),
    }
}
