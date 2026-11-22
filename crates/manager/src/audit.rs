//! Append-only operator audit log.
//!
//! Disabled when `notify.audit_log` is empty. Failures are logged and never
//! fail the request that triggered them.

use std::fs::OpenOptions;
use std::io::Write;

use chrono::Utc;

/// Record one operator action. `detail` should not contain secrets.
pub fn record(path: &str, action: &str, detail: &str) {
    if path.is_empty() {
        return;
    }
    let line = serde_json::json!({
        "time": Utc::now().to_rfc3339(),
        "action": action,
        "detail": detail,
    });
    match OpenOptions::new().create(true).append(true).open(path) {
        Ok(mut file) => {
            if let Err(error) = writeln!(file, "{line}") {
                tracing::warn!(path, %error, "audit log write failed");
            }
        }
        Err(error) => tracing::warn!(path, %error, "audit log open failed"),
    }
}

/// Return the newest audit lines, oldest-first within the window.
/// Missing file or disabled path yields an empty list.
pub fn recent(path: &str, limit: usize) -> Vec<serde_json::Value> {
    if path.is_empty() || limit == 0 {
        return Vec::new();
    }
    let Ok(content) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    content
        .lines()
        .rev()
        .take(limit)
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect()
}
