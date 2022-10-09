use chrono::prelude::*;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs() as i64
}

pub fn strftime(time: i64) -> String {
    format!(
        "{}",
        DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(time.clone(), 0), Utc)
            .format("%Y-%m-%d %H:%M")
    )
}
