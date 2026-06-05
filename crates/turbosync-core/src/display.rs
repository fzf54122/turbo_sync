use chrono::{DateTime, Local};

#[must_use]
pub fn format_bytes(bytes: i64) -> String {
    let bytes = bytes.max(0) as f64;
    if bytes < 1024.0 {
        format!("{} B", bytes as i64)
    } else if bytes < 1024.0 * 1024.0 {
        format!("{:.1} KB", bytes / 1024.0)
    } else if bytes < 1024.0 * 1024.0 * 1024.0 {
        format!("{:.1} MB", bytes / (1024.0 * 1024.0))
    } else {
        format!("{:.1} GB", bytes / (1024.0 * 1024.0 * 1024.0))
    }
}

#[must_use]
pub fn format_unix_time(value: Option<&str>, running_label: &str) -> String {
    let Some(value) = value else {
        return running_label.to_owned();
    };
    let Ok(timestamp) = value.parse::<i64>() else {
        return value.to_owned();
    };

    let Some(datetime) = DateTime::from_timestamp(timestamp, 0) else {
        return value.to_owned();
    };
    let local = datetime.with_timezone(&Local);
    let today = Local::now().date_naive();
    if local.date_naive() == today {
        format!("今天 {}", local.format("%H:%M"))
    } else {
        local.format("%Y-%m-%d %H:%M").to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_bytes_uses_units() {
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1509), "1.5 KB");
        assert_eq!(format_bytes(134_217_728), "128.0 MB");
    }

    #[test]
    fn format_unix_time_handles_missing_and_invalid_values() {
        assert_eq!(format_unix_time(None, "running"), "running");
        assert_eq!(
            format_unix_time(Some("not-a-time"), "running"),
            "not-a-time"
        );
    }
}
