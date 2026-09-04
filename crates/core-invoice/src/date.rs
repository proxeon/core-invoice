//! Calendar [`Date`]: `YYYY-MM-DD`, no time, no timezone. Invalid input fails closed.

use crate::error::DateError;
use std::fmt;

/// Calendar day. No time, no timezone. Inbound `xs:date` offsets are dropped in formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Date {
    year: i32,
    month: u8,
    day: u8,
}

impl Date {
    /// Calendar day. Invalid Y-M-D is `Err`. Year `0..=9999`.
    pub fn new(year: i32, month: u8, day: u8) -> Result<Self, DateError> {
        if !(0..=9999).contains(&year) || !(1..=12).contains(&month) {
            return Err(DateError::Invalid);
        }
        if day == 0 || day > days_in_month(year, month) {
            return Err(DateError::Invalid);
        }
        Ok(Self { year, month, day })
    }

    /// `YYYY-MM-DD` only. Time and zone suffixes fail closed.
    pub fn parse(s: &str) -> Result<Self, DateError> {
        let s = s.trim();
        if s.len() < 10 || s.as_bytes().get(4) != Some(&b'-') || s.as_bytes().get(7) != Some(&b'-')
        {
            return Err(DateError::Invalid);
        }
        if s.len() > 10 {
            // Time of day is forbidden on the type. Zone suffixes are formats' job.
            return Err(DateError::Invalid);
        }
        let year: i32 = s[..4].parse().map_err(|_| DateError::Invalid)?;
        let month: u8 = s[5..7].parse().map_err(|_| DateError::Invalid)?;
        let day: u8 = s[8..10].parse().map_err(|_| DateError::Invalid)?;
        Self::new(year, month, day)
    }

    /// Year (`0..=9999`).
    pub fn year(self) -> i32 {
        self.year
    }
    /// Month (`1..=12`).
    pub fn month(self) -> u8 {
        self.month
    }
    /// Day of month.
    pub fn day(self) -> u8 {
        self.day
    }
}

fn days_in_month(year: i32, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap(year) => 29,
        2 => 28,
        _ => 0,
    }
}

fn leap(year: i32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_iso_day() {
        let d = Date::parse("2026-06-30").unwrap();
        assert_eq!(d.to_string(), "2026-06-30");
        assert!(Date::parse("2026-02-30").is_err());
        assert!(Date::parse("2026-06-01T00:00:00").is_err());
        // Zone suffix is rejected; we do not apply an offset and shift the day.
        assert!(Date::parse("2026-01-15Z").is_err());
        assert!(Date::parse("2026-01-15+00:00").is_err());
    }
}
