use std::fmt;
use std::str::FromStr;

use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A date that may only be known to the year or month: `2006`, `2013-05` or `2015-05-15`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PartialDate {
    pub year: u16,
    pub month: Option<u8>,
    pub day: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidDate(String);

impl fmt::Display for InvalidDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "\"{}\" is not a valid date; use YYYY, YYYY-MM or YYYY-MM-DD",
            self.0
        )
    }
}

impl std::error::Error for InvalidDate {}

impl FromStr for PartialDate {
    type Err = InvalidDate;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let invalid = || InvalidDate(s.to_owned());
        let number = |part: &str, len: usize| {
            if part.len() == len && part.bytes().all(|b| b.is_ascii_digit()) {
                part.parse::<u16>().ok()
            } else {
                None
            }
        };

        let mut parts = s.split('-');
        let year = parts
            .next()
            .and_then(|p| number(p, 4))
            .ok_or_else(invalid)?;
        let month = parts
            .next()
            .map(|p| {
                number(p, 2)
                    .filter(|m| (1..=12).contains(m))
                    .ok_or_else(invalid)
            })
            .transpose()?;
        let day = match month {
            None => None,
            Some(m) => parts
                .next()
                .map(|p| {
                    number(p, 2)
                        .filter(|d| (1..=days_in_month(year, m)).contains(d))
                        .ok_or_else(invalid)
                })
                .transpose()?,
        };
        if parts.next().is_some() {
            return Err(invalid());
        }
        Ok(PartialDate {
            year,
            month: month.map(|m| m as u8),
            day: day.map(|d| d as u8),
        })
    }
}

fn days_in_month(year: u16, month: u16) -> u16 {
    match month {
        2 if year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400)) => {
            29
        }
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

impl fmt::Display for PartialDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}", self.year)?;
        if let Some(month) = self.month {
            write!(f, "-{month:02}")?;
        }
        if let Some(day) = self.day {
            write!(f, "-{day:02}")?;
        }
        Ok(())
    }
}

impl Serialize for PartialDate {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for PartialDate {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct DateVisitor;

        impl Visitor<'_> for DateVisitor {
            type Value = PartialDate;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a date like 2015, 2015-05 or 2015-05-15")
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<PartialDate, E> {
                v.parse().map_err(E::custom)
            }

            // An unquoted year such as `2006` arrives as a number.
            fn visit_u64<E: de::Error>(self, v: u64) -> Result<PartialDate, E> {
                self.visit_str(&v.to_string())
            }

            fn visit_i64<E: de::Error>(self, v: i64) -> Result<PartialDate, E> {
                self.visit_str(&v.to_string())
            }
        }

        deserializer.deserialize_any(DateVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_years_months_and_days() {
        let date = |s: &str| s.parse::<PartialDate>().map(|d| d.to_string());
        assert_eq!(date("2006"), Ok("2006".into()));
        assert_eq!(date("2013-05"), Ok("2013-05".into()));
        assert_eq!(date("2024-02-29"), Ok("2024-02-29".into()));
    }

    #[test]
    fn rejects_impossible_dates() {
        for s in [
            "06",
            "2013-13",
            "2023-02-29",
            "2015-5-15",
            "2015-05-15-01",
            "2015--01",
        ] {
            assert!(s.parse::<PartialDate>().is_err(), "{s} should be rejected");
        }
    }
}
