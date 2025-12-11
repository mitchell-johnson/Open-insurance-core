//! Bi-temporal data handling types
//!
//! This module provides types for managing bi-temporal data:
//! - Valid Time: When a fact is true in the real world
//! - System Time: When the fact was recorded in the database

use chrono::{DateTime, NaiveDate, Utc};
use chrono_tz::Tz;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use thiserror::Error;
use std::str::FromStr;

/// Timezone wrapper for policy jurisdictions
///
/// Wraps chrono_tz::Tz with custom serialization support.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timezone(pub Tz);

impl Serialize for Timezone {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.0.name())
    }
}

impl<'de> Deserialize<'de> for Timezone {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Tz::from_str(&s)
            .map(Timezone)
            .map_err(|_| serde::de::Error::custom(format!("Invalid timezone: {}", s)))
    }
}

impl Timezone {
    pub fn new(tz: Tz) -> Self {
        Self(tz)
    }

    /// Converts a UTC datetime to the local timezone
    pub fn to_local(&self, utc: DateTime<Utc>) -> DateTime<Tz> {
        utc.with_timezone(&self.0)
    }

    /// Gets the start of day (00:00:00) in this timezone as UTC
    pub fn start_of_day(&self, date: NaiveDate) -> DateTime<Utc> {
        date.and_hms_opt(0, 0, 0)
            .unwrap()
            .and_local_timezone(self.0)
            .single()
            .expect("Invalid timezone conversion")
            .with_timezone(&Utc)
    }

    /// Gets the end of day (23:59:59.999999999) in this timezone as UTC
    pub fn end_of_day(&self, date: NaiveDate) -> DateTime<Utc> {
        date.and_hms_nano_opt(23, 59, 59, 999_999_999)
            .unwrap()
            .and_local_timezone(self.0)
            .single()
            .expect("Invalid timezone conversion")
            .with_timezone(&Utc)
    }
}

impl Default for Timezone {
    fn default() -> Self {
        Self(chrono_tz::UTC)
    }
}

/// Errors related to temporal operations
#[derive(Debug, Error, PartialEq, Eq)]
pub enum TemporalError {
    #[error("Invalid period: start {start} must be before end {end}")]
    InvalidPeriod {
        start: String,
        end: String,
    },

    #[error("Periods overlap")]
    PeriodsOverlap,

    #[error("Gap in temporal sequence")]
    GapInSequence,

    #[error("Period is in the future")]
    FuturePeriod,
}

/// Represents a valid time period (when a fact is true in the real world)
///
/// This is used to track the business-effective dates of data.
/// For example, a policy coverage might be valid from Jan 1 to Dec 31.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidPeriod {
    /// Start of the valid period (inclusive)
    pub start: DateTime<Utc>,
    /// End of the valid period (exclusive), None means unbounded
    pub end: Option<DateTime<Utc>>,
}

impl ValidPeriod {
    /// Creates a new valid period
    pub fn new(start: DateTime<Utc>, end: Option<DateTime<Utc>>) -> Result<Self, TemporalError> {
        if let Some(end) = end {
            if start >= end {
                return Err(TemporalError::InvalidPeriod {
                    start: start.to_string(),
                    end: end.to_string(),
                });
            }
        }
        Ok(Self { start, end })
    }

    /// Creates an unbounded period starting from the given time
    pub fn from(start: DateTime<Utc>) -> Self {
        Self { start, end: None }
    }

    /// Creates a bounded period
    pub fn bounded(start: DateTime<Utc>, end: DateTime<Utc>) -> Result<Self, TemporalError> {
        Self::new(start, Some(end))
    }

    /// Returns true if this period contains the given timestamp
    pub fn contains(&self, timestamp: DateTime<Utc>) -> bool {
        timestamp >= self.start && self.end.map_or(true, |e| timestamp < e)
    }

    /// Returns true if this period overlaps with another
    pub fn overlaps(&self, other: &ValidPeriod) -> bool {
        let self_end = self.end.unwrap_or(DateTime::<Utc>::MAX_UTC);
        let other_end = other.end.unwrap_or(DateTime::<Utc>::MAX_UTC);

        self.start < other_end && other.start < self_end
    }

    /// Returns true if this period is unbounded (no end date)
    pub fn is_unbounded(&self) -> bool {
        self.end.is_none()
    }

    /// Closes the period at the given timestamp
    pub fn close_at(&mut self, timestamp: DateTime<Utc>) -> Result<(), TemporalError> {
        if timestamp <= self.start {
            return Err(TemporalError::InvalidPeriod {
                start: self.start.to_string(),
                end: timestamp.to_string(),
            });
        }
        self.end = Some(timestamp);
        Ok(())
    }

    /// Returns the duration of the period, if bounded
    pub fn duration(&self) -> Option<chrono::Duration> {
        self.end.map(|e| e - self.start)
    }
}

/// Represents a system time period (when a fact was recorded in the database)
///
/// This is used for audit trails to track when data was entered or modified.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemPeriod {
    /// When the record was created
    pub recorded_at: DateTime<Utc>,
    /// When the record was superseded (None means current)
    pub superseded_at: Option<DateTime<Utc>>,
}

impl SystemPeriod {
    /// Creates a new current system period
    pub fn current() -> Self {
        Self {
            recorded_at: Utc::now(),
            superseded_at: None,
        }
    }

    /// Creates a system period with a specific recorded time
    pub fn recorded_at(timestamp: DateTime<Utc>) -> Self {
        Self {
            recorded_at: timestamp,
            superseded_at: None,
        }
    }

    /// Returns true if this is the current (non-superseded) record
    pub fn is_current(&self) -> bool {
        self.superseded_at.is_none()
    }

    /// Supersedes this record at the given timestamp
    pub fn supersede(&mut self) {
        self.superseded_at = Some(Utc::now());
    }

    /// Supersedes this record at a specific timestamp
    pub fn supersede_at(&mut self, timestamp: DateTime<Utc>) {
        self.superseded_at = Some(timestamp);
    }

    /// Returns true if this record was active at the given system time
    pub fn active_at(&self, timestamp: DateTime<Utc>) -> bool {
        timestamp >= self.recorded_at
            && self.superseded_at.map_or(true, |s| timestamp < s)
    }
}

/// A bi-temporal record combining valid time and system time
///
/// This struct wraps any data type with bi-temporal metadata,
/// enabling "time travel" queries:
/// - What did we think was true at time T about time V?
/// - What is currently known about time V?
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiTemporalRecord<T> {
    /// The actual data
    pub data: T,
    /// When this data is/was true in the real world
    pub valid_period: ValidPeriod,
    /// When this data was recorded/superseded in the system
    pub system_period: SystemPeriod,
}

impl<T> BiTemporalRecord<T> {
    /// Creates a new bi-temporal record with current system time
    pub fn new(data: T, valid_period: ValidPeriod) -> Self {
        Self {
            data,
            valid_period,
            system_period: SystemPeriod::current(),
        }
    }

    /// Creates a record that starts being valid now
    pub fn effective_now(data: T) -> Self {
        Self {
            data,
            valid_period: ValidPeriod::from(Utc::now()),
            system_period: SystemPeriod::current(),
        }
    }

    /// Returns true if this record is the current version
    pub fn is_current(&self) -> bool {
        self.system_period.is_current()
    }

    /// Returns true if this record is valid at the given time
    pub fn valid_at(&self, timestamp: DateTime<Utc>) -> bool {
        self.valid_period.contains(timestamp)
    }

    /// Returns true if this record was known at the given system time
    pub fn known_at(&self, timestamp: DateTime<Utc>) -> bool {
        self.system_period.active_at(timestamp)
    }

    /// Returns true if this record was valid at valid_time and known at system_time
    pub fn effective_at(&self, valid_time: DateTime<Utc>, system_time: DateTime<Utc>) -> bool {
        self.valid_at(valid_time) && self.known_at(system_time)
    }

    /// Supersedes this record with a new version
    pub fn supersede(&mut self) {
        self.system_period.supersede();
    }
}

/// Represents a date range for policy periods
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DateRange {
    pub start: NaiveDate,
    pub end: NaiveDate,
}

impl DateRange {
    pub fn new(start: NaiveDate, end: NaiveDate) -> Result<Self, TemporalError> {
        if start > end {
            return Err(TemporalError::InvalidPeriod {
                start: start.to_string(),
                end: end.to_string(),
            });
        }
        Ok(Self { start, end })
    }

    pub fn contains(&self, date: NaiveDate) -> bool {
        date >= self.start && date <= self.end
    }

    pub fn days(&self) -> i64 {
        (self.end - self.start).num_days()
    }

    /// Converts to ValidPeriod using the given timezone
    pub fn to_valid_period(&self, tz: &Timezone) -> ValidPeriod {
        ValidPeriod {
            start: tz.start_of_day(self.start),
            end: Some(tz.end_of_day(self.end)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_valid_period_creation() {
        let start = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2024, 12, 31, 23, 59, 59).unwrap();

        let period = ValidPeriod::bounded(start, end).unwrap();
        assert!(period.contains(Utc.with_ymd_and_hms(2024, 6, 15, 12, 0, 0).unwrap()));
        assert!(!period.contains(Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap()));
    }

    #[test]
    fn test_valid_period_overlap() {
        let p1 = ValidPeriod::bounded(
            Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
            Utc.with_ymd_and_hms(2024, 6, 30, 0, 0, 0).unwrap(),
        ).unwrap();

        let p2 = ValidPeriod::bounded(
            Utc.with_ymd_and_hms(2024, 6, 1, 0, 0, 0).unwrap(),
            Utc.with_ymd_and_hms(2024, 12, 31, 0, 0, 0).unwrap(),
        ).unwrap();

        assert!(p1.overlaps(&p2));
    }

    #[test]
    fn test_system_period_current() {
        let sp = SystemPeriod::current();
        assert!(sp.is_current());
    }

    #[test]
    fn test_bitemporal_record() {
        let data = "Test data";
        let valid_period = ValidPeriod::from(Utc::now());
        let record = BiTemporalRecord::new(data, valid_period);

        assert!(record.is_current());
        assert!(record.valid_at(Utc::now()));
    }
}
