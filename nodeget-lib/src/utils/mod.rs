pub fn get_local_timestamp_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};

    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(std::time::Duration::ZERO);
    let millis = duration.as_millis();
    u64::try_from(millis).unwrap_or(0)
}