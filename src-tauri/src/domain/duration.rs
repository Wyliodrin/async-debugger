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
        let formatted = Self::get_formatted_duration(seconds, nanos);
        Self {
            seconds,
            nanos,
            formatted,
        }
    }

    /// Returns a human‐readable duration for the given seconds and nanoseconds count.
    ///
    /// The output format adapts to the magnitude of the duration:
    ///
    /// - If `seconds >= 3600`: shows hours and minutes (e.g., `"1h 30min"`)
    /// - If `seconds >= 60`: shows minutes and seconds (e.g., `"3min 45s"`)
    /// - If `seconds > 0`: shows seconds and a sub-second unit (e.g., `"2s 500ms"`)
    /// - If `seconds == 0`: shows only the sub-second unit:
    ///     - `>= 1_000_000` nanoseconds → milliseconds, rounded
    ///     - `>= 1_000` nanoseconds → microseconds, rounded
    ///     - `< 1_000` nanoseconds → nanoseconds
    ///
    /// # Arguments
    ///
    /// * `seconds` - Number of whole seconds
    /// * `nanos` - Number of nanoseconds (0–999_999_999)
    ///
    /// # Returns
    ///
    /// A `String` representing the duration in a human-readable format.
    ///
    /// # Examples
    ///
    /// ```
    /// use your_crate::domain::durations::Duration;
    ///
    /// assert_eq!(Duration::get_formatted_duration(0, 2_500_000), "3ms");
    /// assert_eq!(Duration::get_formatted_duration(0, 2_500), "3μs");
    /// assert_eq!(Duration::get_formatted_duration(0, 250), "250ns");
    /// assert_eq!(Duration::get_formatted_duration(75, 1_200_000), "1min 15s");
    /// assert_eq!(Duration::get_formatted_duration(3_780, 500_000), "1h 3min");
    /// assert_eq!(Duration::get_formatted_duration(2, 1_500_000), "2s 2ms");
    /// ```
    pub fn get_formatted_duration(seconds: i64, nanos: i32) -> String {
        if seconds > 3_600 {
            let hours = ((seconds as f64) / 3_600.0).floor() as i64;
            return format!(
                "{}h {}min",
                hours,
                (((seconds - hours * 3_600) as f64) / 60.0).round() as i64
            );
        }

        if seconds > 60 {
            let minutes = ((seconds as f64) / 60.0).floor() as i64;
            return format!("{}min {}s", minutes, seconds - minutes * 60);
        }

        let sub_seconds_format;
        if nanos >= 1_000_000 {
            sub_seconds_format = format!("{}ms", ((nanos as f64) / 1_000_000.0).round() as i32);
        } else if nanos >= 1_000 {
            sub_seconds_format = format!("{}μs", ((nanos as f64) / 1_000.0).round() as i32);
        } else {
            sub_seconds_format = format!("{}ns", nanos);
        }

        if seconds > 0 {
            return format!("{}s {}", seconds, sub_seconds_format);
        } else {
            return sub_seconds_format;
        }
    }
}
