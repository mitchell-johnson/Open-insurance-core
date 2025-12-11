//! Comprehensive unit tests for the Temporal module
//!
//! Tests cover ValidPeriod, SystemPeriod, BiTemporalRecord,
//! DateRange, and Timezone functionality.

use core_kernel::{ValidPeriod, SystemPeriod, BiTemporalRecord, Timezone};
use core_kernel::temporal::{TemporalError, DateRange};
use chrono::{DateTime, Duration, NaiveDate, TimeZone, Timelike, Utc};

mod valid_period {
    use super::*;

    mod creation {
        use super::*;

        #[test]
        fn test_new_creates_valid_period() {
            let start = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
            let end = Utc.with_ymd_and_hms(2024, 12, 31, 23, 59, 59).unwrap();
            let period = ValidPeriod::new(start, Some(end)).unwrap();

            assert_eq!(period.start, start);
            assert_eq!(period.end, Some(end));
        }

        #[test]
        fn test_new_with_none_end_is_unbounded() {
            let start = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
            let period = ValidPeriod::new(start, None).unwrap();

            assert!(period.is_unbounded());
        }

        #[test]
        fn test_new_fails_when_start_after_end() {
            let start = Utc.with_ymd_and_hms(2024, 12, 31, 0, 0, 0).unwrap();
            let end = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
            let result = ValidPeriod::new(start, Some(end));

            assert!(matches!(result, Err(TemporalError::InvalidPeriod { .. })));
        }

        #[test]
        fn test_new_fails_when_start_equals_end() {
            let start = Utc.with_ymd_and_hms(2024, 6, 15, 12, 0, 0).unwrap();
            let result = ValidPeriod::new(start, Some(start));

            assert!(matches!(result, Err(TemporalError::InvalidPeriod { .. })));
        }

        #[test]
        fn test_from_creates_unbounded_period() {
            let start = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
            let period = ValidPeriod::from(start);

            assert_eq!(period.start, start);
            assert!(period.is_unbounded());
        }

        #[test]
        fn test_bounded_creates_bounded_period() {
            let start = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
            let end = Utc.with_ymd_and_hms(2024, 12, 31, 23, 59, 59).unwrap();
            let period = ValidPeriod::bounded(start, end).unwrap();

            assert!(!period.is_unbounded());
            assert_eq!(period.end, Some(end));
        }
    }

    mod containment {
        use super::*;

        #[test]
        fn test_contains_timestamp_in_middle() {
            let start = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
            let end = Utc.with_ymd_and_hms(2024, 12, 31, 23, 59, 59).unwrap();
            let period = ValidPeriod::bounded(start, end).unwrap();

            let mid = Utc.with_ymd_and_hms(2024, 6, 15, 12, 0, 0).unwrap();
            assert!(period.contains(mid));
        }

        #[test]
        fn test_contains_start_timestamp() {
            let start = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
            let end = Utc.with_ymd_and_hms(2024, 12, 31, 23, 59, 59).unwrap();
            let period = ValidPeriod::bounded(start, end).unwrap();

            assert!(period.contains(start));
        }

        #[test]
        fn test_contains_excludes_end_timestamp() {
            let start = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
            let end = Utc.with_ymd_and_hms(2024, 12, 31, 23, 59, 59).unwrap();
            let period = ValidPeriod::bounded(start, end).unwrap();

            assert!(!period.contains(end));
        }

        #[test]
        fn test_contains_excludes_before_start() {
            let start = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
            let end = Utc.with_ymd_and_hms(2024, 12, 31, 23, 59, 59).unwrap();
            let period = ValidPeriod::bounded(start, end).unwrap();

            let before = Utc.with_ymd_and_hms(2023, 12, 31, 0, 0, 0).unwrap();
            assert!(!period.contains(before));
        }

        #[test]
        fn test_unbounded_contains_far_future() {
            let start = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
            let period = ValidPeriod::from(start);

            let far_future = Utc.with_ymd_and_hms(2100, 12, 31, 23, 59, 59).unwrap();
            assert!(period.contains(far_future));
        }
    }

    mod overlap {
        use super::*;

        #[test]
        fn test_overlapping_periods() {
            let p1 = ValidPeriod::bounded(
                Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
                Utc.with_ymd_and_hms(2024, 6, 30, 0, 0, 0).unwrap(),
            ).unwrap();

            let p2 = ValidPeriod::bounded(
                Utc.with_ymd_and_hms(2024, 6, 1, 0, 0, 0).unwrap(),
                Utc.with_ymd_and_hms(2024, 12, 31, 0, 0, 0).unwrap(),
            ).unwrap();

            assert!(p1.overlaps(&p2));
            assert!(p2.overlaps(&p1));
        }

        #[test]
        fn test_non_overlapping_periods() {
            let p1 = ValidPeriod::bounded(
                Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
                Utc.with_ymd_and_hms(2024, 6, 1, 0, 0, 0).unwrap(),
            ).unwrap();

            let p2 = ValidPeriod::bounded(
                Utc.with_ymd_and_hms(2024, 6, 1, 0, 0, 0).unwrap(),
                Utc.with_ymd_and_hms(2024, 12, 31, 0, 0, 0).unwrap(),
            ).unwrap();

            // Adjacent periods (end of p1 equals start of p2) don't overlap
            assert!(!p1.overlaps(&p2));
        }

        #[test]
        fn test_contained_period_overlaps() {
            let outer = ValidPeriod::bounded(
                Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
                Utc.with_ymd_and_hms(2024, 12, 31, 0, 0, 0).unwrap(),
            ).unwrap();

            let inner = ValidPeriod::bounded(
                Utc.with_ymd_and_hms(2024, 3, 1, 0, 0, 0).unwrap(),
                Utc.with_ymd_and_hms(2024, 9, 30, 0, 0, 0).unwrap(),
            ).unwrap();

            assert!(outer.overlaps(&inner));
            assert!(inner.overlaps(&outer));
        }

        #[test]
        fn test_unbounded_periods_overlap() {
            let p1 = ValidPeriod::from(Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap());
            let p2 = ValidPeriod::from(Utc.with_ymd_and_hms(2024, 6, 1, 0, 0, 0).unwrap());

            assert!(p1.overlaps(&p2));
        }
    }

    mod close_at {
        use super::*;

        #[test]
        fn test_close_at_success() {
            let start = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
            let mut period = ValidPeriod::from(start);

            let close_time = Utc.with_ymd_and_hms(2024, 6, 30, 0, 0, 0).unwrap();
            period.close_at(close_time).unwrap();

            assert_eq!(period.end, Some(close_time));
            assert!(!period.is_unbounded());
        }

        #[test]
        fn test_close_at_before_start_fails() {
            let start = Utc.with_ymd_and_hms(2024, 6, 1, 0, 0, 0).unwrap();
            let mut period = ValidPeriod::from(start);

            let close_time = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
            let result = period.close_at(close_time);

            assert!(matches!(result, Err(TemporalError::InvalidPeriod { .. })));
        }

        #[test]
        fn test_close_at_equal_to_start_fails() {
            let start = Utc.with_ymd_and_hms(2024, 6, 1, 0, 0, 0).unwrap();
            let mut period = ValidPeriod::from(start);

            let result = period.close_at(start);
            assert!(matches!(result, Err(TemporalError::InvalidPeriod { .. })));
        }
    }

    mod duration {
        use super::*;

        #[test]
        fn test_duration_bounded_period() {
            let start = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
            let end = Utc.with_ymd_and_hms(2024, 1, 31, 0, 0, 0).unwrap();
            let period = ValidPeriod::bounded(start, end).unwrap();

            let duration = period.duration().unwrap();
            assert_eq!(duration.num_days(), 30);
        }

        #[test]
        fn test_duration_unbounded_returns_none() {
            let start = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
            let period = ValidPeriod::from(start);

            assert!(period.duration().is_none());
        }
    }
}

mod system_period {
    use super::*;

    #[test]
    fn test_current_creates_unsuperseded_record() {
        let sp = SystemPeriod::current();
        assert!(sp.is_current());
        assert!(sp.superseded_at.is_none());
    }

    #[test]
    fn test_recorded_at_creates_with_timestamp() {
        let timestamp = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
        let sp = SystemPeriod::recorded_at(timestamp);

        assert_eq!(sp.recorded_at, timestamp);
        assert!(sp.is_current());
    }

    #[test]
    fn test_supersede_marks_as_not_current() {
        let mut sp = SystemPeriod::current();
        sp.supersede();

        assert!(!sp.is_current());
        assert!(sp.superseded_at.is_some());
    }

    #[test]
    fn test_supersede_at_specific_time() {
        let mut sp = SystemPeriod::current();
        let supersede_time = Utc.with_ymd_and_hms(2024, 6, 15, 12, 0, 0).unwrap();
        sp.supersede_at(supersede_time);

        assert_eq!(sp.superseded_at, Some(supersede_time));
    }

    #[test]
    fn test_active_at_current_record() {
        let sp = SystemPeriod::current();
        let now = Utc::now();

        assert!(sp.active_at(now));
    }

    #[test]
    fn test_active_at_before_recorded() {
        let recorded = Utc.with_ymd_and_hms(2024, 6, 1, 0, 0, 0).unwrap();
        let sp = SystemPeriod::recorded_at(recorded);

        let before = Utc.with_ymd_and_hms(2024, 5, 1, 0, 0, 0).unwrap();
        assert!(!sp.active_at(before));
    }

    #[test]
    fn test_active_at_after_superseded() {
        let recorded = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
        let superseded = Utc.with_ymd_and_hms(2024, 6, 1, 0, 0, 0).unwrap();

        let mut sp = SystemPeriod::recorded_at(recorded);
        sp.supersede_at(superseded);

        let after = Utc.with_ymd_and_hms(2024, 7, 1, 0, 0, 0).unwrap();
        assert!(!sp.active_at(after));
    }

    #[test]
    fn test_active_at_between_recorded_and_superseded() {
        let recorded = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
        let superseded = Utc.with_ymd_and_hms(2024, 6, 1, 0, 0, 0).unwrap();

        let mut sp = SystemPeriod::recorded_at(recorded);
        sp.supersede_at(superseded);

        let between = Utc.with_ymd_and_hms(2024, 3, 15, 0, 0, 0).unwrap();
        assert!(sp.active_at(between));
    }
}

mod bitemporal_record {
    use super::*;

    #[test]
    fn test_new_creates_current_record() {
        let data = "test data";
        let valid_period = ValidPeriod::from(Utc::now());
        let record = BiTemporalRecord::new(data, valid_period);

        assert!(record.is_current());
        assert_eq!(record.data, "test data");
    }

    #[test]
    fn test_effective_now_creates_record_valid_from_now() {
        let record = BiTemporalRecord::effective_now("test");

        assert!(record.is_current());
        assert!(record.valid_at(Utc::now()));
    }

    #[test]
    fn test_valid_at() {
        let start = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2024, 12, 31, 0, 0, 0).unwrap();
        let valid_period = ValidPeriod::bounded(start, end).unwrap();
        let record = BiTemporalRecord::new("test", valid_period);

        let mid = Utc.with_ymd_and_hms(2024, 6, 15, 0, 0, 0).unwrap();
        assert!(record.valid_at(mid));

        let before = Utc.with_ymd_and_hms(2023, 6, 15, 0, 0, 0).unwrap();
        assert!(!record.valid_at(before));
    }

    #[test]
    fn test_known_at() {
        let record = BiTemporalRecord::effective_now("test");

        assert!(record.known_at(Utc::now()));

        let past = Utc.with_ymd_and_hms(2020, 1, 1, 0, 0, 0).unwrap();
        assert!(!record.known_at(past));
    }

    #[test]
    fn test_effective_at() {
        let start = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
        let valid_period = ValidPeriod::from(start);
        let record = BiTemporalRecord::new("test", valid_period);

        let mid = Utc.with_ymd_and_hms(2024, 6, 15, 0, 0, 0).unwrap();
        let now = Utc::now();

        assert!(record.effective_at(mid, now));
    }

    #[test]
    fn test_supersede() {
        let record_data = "test";
        let valid_period = ValidPeriod::from(Utc::now());
        let mut record = BiTemporalRecord::new(record_data, valid_period);

        assert!(record.is_current());
        record.supersede();
        assert!(!record.is_current());
    }
}

mod date_range {
    use super::*;

    #[test]
    fn test_new_creates_valid_range() {
        let start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 12, 31).unwrap();
        let range = DateRange::new(start, end).unwrap();

        assert_eq!(range.start, start);
        assert_eq!(range.end, end);
    }

    #[test]
    fn test_new_same_start_end_is_valid() {
        let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        let range = DateRange::new(date, date).unwrap();

        assert_eq!(range.days(), 0);
    }

    #[test]
    fn test_new_fails_when_start_after_end() {
        let start = NaiveDate::from_ymd_opt(2024, 12, 31).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let result = DateRange::new(start, end);

        assert!(matches!(result, Err(TemporalError::InvalidPeriod { .. })));
    }

    #[test]
    fn test_contains_date_in_range() {
        let start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 12, 31).unwrap();
        let range = DateRange::new(start, end).unwrap();

        let mid = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        assert!(range.contains(mid));
    }

    #[test]
    fn test_contains_start_date() {
        let start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 12, 31).unwrap();
        let range = DateRange::new(start, end).unwrap();

        assert!(range.contains(start));
    }

    #[test]
    fn test_contains_end_date() {
        let start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 12, 31).unwrap();
        let range = DateRange::new(start, end).unwrap();

        assert!(range.contains(end));
    }

    #[test]
    fn test_excludes_date_before_range() {
        let start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 12, 31).unwrap();
        let range = DateRange::new(start, end).unwrap();

        let before = NaiveDate::from_ymd_opt(2023, 12, 31).unwrap();
        assert!(!range.contains(before));
    }

    #[test]
    fn test_days_calculation() {
        let start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 1, 31).unwrap();
        let range = DateRange::new(start, end).unwrap();

        assert_eq!(range.days(), 30);
    }

    #[test]
    fn test_to_valid_period() {
        let start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 12, 31).unwrap();
        let range = DateRange::new(start, end).unwrap();

        let tz = Timezone::default();
        let valid_period = range.to_valid_period(&tz);

        assert!(!valid_period.is_unbounded());
    }
}

mod timezone {
    use super::*;
    use chrono_tz;

    #[test]
    fn test_default_is_utc() {
        let tz = Timezone::default();
        assert_eq!(tz.0, chrono_tz::UTC);
    }

    #[test]
    fn test_new_creates_timezone() {
        let tz = Timezone::new(chrono_tz::America::New_York);
        assert_eq!(tz.0, chrono_tz::America::New_York);
    }

    #[test]
    fn test_to_local_conversion() {
        let tz = Timezone::new(chrono_tz::UTC);
        let utc_time = Utc.with_ymd_and_hms(2024, 6, 15, 12, 0, 0).unwrap();
        let local = tz.to_local(utc_time);

        assert_eq!(local.hour(), 12);
    }

    #[test]
    fn test_start_of_day() {
        let tz = Timezone::default();
        let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        let start = tz.start_of_day(date);

        assert_eq!(start.hour(), 0);
        assert_eq!(start.minute(), 0);
        assert_eq!(start.second(), 0);
    }

    #[test]
    fn test_end_of_day() {
        let tz = Timezone::default();
        let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        let end = tz.end_of_day(date);

        assert_eq!(end.hour(), 23);
        assert_eq!(end.minute(), 59);
        assert_eq!(end.second(), 59);
    }
}

mod serialization {
    use super::*;
    use serde_json;

    #[test]
    fn test_valid_period_json_roundtrip() {
        let start = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2024, 12, 31, 23, 59, 59).unwrap();
        let period = ValidPeriod::bounded(start, end).unwrap();

        let json = serde_json::to_string(&period).unwrap();
        let deserialized: ValidPeriod = serde_json::from_str(&json).unwrap();

        assert_eq!(period.start, deserialized.start);
        assert_eq!(period.end, deserialized.end);
    }

    #[test]
    fn test_bitemporal_record_json_roundtrip() {
        let data = "test data";
        let valid_period = ValidPeriod::from(Utc::now());
        let record = BiTemporalRecord::new(data.to_string(), valid_period);

        let json = serde_json::to_string(&record).unwrap();
        let deserialized: BiTemporalRecord<String> = serde_json::from_str(&json).unwrap();

        assert_eq!(record.data, deserialized.data);
    }
}
