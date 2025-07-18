//! This module defines a human‐readable `Duration` type which carries both
//! raw seconds/nanos and a formatted sub‐second string.

use serde::{Deserialize, Serialize};

/// A wrapper around a seconds + nanoseconds duration plus a human‐readable label.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Duration {
    /// Whole seconds part of the duration.
    pub seconds: i64,
    /// Nanoseconds part of the duration.
    pub nanos: i32,
    /// Pre‐computed formatted sub‐second string (e.g. "15ms", "200μs", "50ns").
    pub formatted: String,
}

impl Duration {
    /// Creates a new `Duration` from raw seconds and nanoseconds.
    ///
    /// The `formatted` field is computed automatically via
    /// [`get_correct_subdivision_sec`].
    ///
    /// # Examples
    ///
    /// ```
    /// use your_crate::domain::durations::Duration;
    /// let d = Duration::new(1, 1_500_000);
    /// assert_eq!(d.formatted, "2ms");
    /// ```
    pub fn new(seconds: i64, nanos: i32) -> Self {
        let formatted = Self::get_correct_subdivision_sec(nanos);
        Self {
            seconds,
            nanos,
            formatted,
        }
    }

    /// Returns a human‐readable sub‐division for the given nanosecond count.
    ///
    /// - `>= 1_000_000` ⇒ milliseconds (rounded)
    /// - `>= 1_000`     ⇒ microseconds (rounded)
    /// - otherwise      ⇒ nanoseconds
    ///
    /// # Examples
    ///
    /// ```
    /// use your_crate::domain::durations::Duration;
    /// assert_eq!(Duration::get_correct_subdivision_sec(2_500_000), "3ms");
    /// assert_eq!(Duration::get_correct_subdivision_sec(2_500), "3μs");
    /// assert_eq!(Duration::get_correct_subdivision_sec(250), "250ns");
    /// ```
    pub fn get_correct_subdivision_sec(nano: i32) -> String {
        if nano >= 1_000_000 {
            return format!("{}ms", ((nano as f64) / 1_000_000.0).round() as i32);
        }

        if nano >= 1_000 {
            return format!("{}μs", ((nano as f64) / 1_000.0).round() as i32);
        }

        format!("{}ns", nano)
    }
}
