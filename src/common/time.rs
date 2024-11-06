use pyo3::prelude::*;
use chrono::{DateTime, NaiveDateTime, Utc, TimeZone};
use std::fmt;

/// Time representation for the Chan system
#[pyclass]
#[derive(Debug, Clone)]
pub struct Time {
    pub year: i32,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
    pub auto: bool,  // 自适应对天的理解
    pub ts: i64,     // Unix timestamp
}

#[pymethods]
impl Time {
    /// Create a new Time instance
    #[new]
    pub fn new(year: i32, month: u32, day: u32, hour: u32, minute: u32, second: u32, auto: bool) -> PyResult<Self> {
        let mut time = Self {
            year,
            month,
            day,
            hour,
            minute,
            second,
            auto,
            ts: 0,
        };
        time.set_timestamp()?;
        Ok(time)
    }

    /// Convert Time to string representation
    fn __str__(&self) -> String {
        self.to_string()
    }

    /// Convert Time to string with optional separator
    #[pyo3(name = "toDateStr")]
    pub fn to_date_str(&self, splt: &str) -> String {
        format!("{:04}{}{:02}{}{:02}", 
            self.year, splt, self.month, splt, self.day)
    }

    /// Create a new Time instance with only date components
    #[pyo3(name = "toDate")]
    pub fn to_date(&self) -> PyResult<Time> {
        Time::new(self.year, self.month, self.day, 0, 0, 0, false)
    }

    /// Compare if this Time is greater than another Time
    fn __gt__(&self, other: &Time) -> bool {
        self.ts > other.ts
    }

    /// Compare if this Time is greater than or equal to another Time
    fn __ge__(&self, other: &Time) -> bool {
        self.ts >= other.ts
    }
}

impl Time {
    /// Set the Unix timestamp based on the time components
    fn set_timestamp(&mut self) -> PyResult<()> {
        let date = if self.hour == 0 && self.minute == 0 && self.auto {
            // When auto is true and time is midnight, use 23:59 of the same day
            NaiveDateTime::new(
                chrono::NaiveDate::from_ymd_opt(self.year, self.month, self.day)
                    .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid date"))?,
                chrono::NaiveTime::from_hms_opt(23, 59, self.second)
                    .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid time"))?
            )
        } else {
            NaiveDateTime::new(
                chrono::NaiveDate::from_ymd_opt(self.year, self.month, self.day)
                    .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid date"))?,
                chrono::NaiveTime::from_hms_opt(self.hour, self.minute, self.second)
                    .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid time"))?
            )
        };

        self.ts = Utc.from_utc_datetime(&date).timestamp();
        Ok(())
    }
}

impl fmt::Display for Time {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.hour == 0 && self.minute == 0 {
            write!(f, "{:04}/{:02}/{:02}", self.year, self.month, self.day)
        } else {
            write!(f, "{:04}/{:02}/{:02} {:02}:{:02}", 
                self.year, self.month, self.day, self.hour, self.minute)
        }
    }
}

// Optional: Implement comparison traits
impl PartialEq for Time {
    fn eq(&self, other: &Self) -> bool {
        self.ts == other.ts
    }
}

impl Eq for Time {}

impl PartialOrd for Time {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.ts.cmp(&other.ts))
    }
}

impl Ord for Time {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.ts.cmp(&other.ts)
    }
} 