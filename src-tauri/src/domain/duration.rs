use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Duration {
    pub seconds: i64,
    pub nanos: i32,
    pub formatted: String,
}

impl Duration {
    pub fn new(seconds: i64, nanos: i32) -> Self {
        let formatted = Self::get_correct_subdivision_sec(nanos);
        Self {
            seconds,
            nanos,
            formatted,
        }
    }

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
