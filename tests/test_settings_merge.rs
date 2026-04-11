use claude_code_sync::sync::merge_settings_json;
use serde_json::{json, Map, Value};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const TS_KEY: &str = "lastModifiedTimestamp";

/// Build a SystemTime from a Unix millisecond timestamp — convenience for tests.
fn sys_time_ms(ms: i64) -> SystemTime {
    UNIX_EPOCH + Duration::from_millis(ms as u64)
}

fn map(v: Value) -> Map<String, Value> {
    v.as_object().cloned().unwrap()
}

// ---------------------------------------------------------------------------
// Rule 2: local has no lastModifiedTimestamp → remote wins
// ---------------------------------------------------------------------------

#[test]
fn test_local_no_timestamp_remote_wins() {
    let local = map(json!({ "theme": "dark" }));
    let remote = map(json!({ "theme": "light", TS_KEY: 1_000_000i64 }));
    let (result, local_wins) = merge_settings_json(&local, &remote, SystemTime::now());

    assert!(!local_wins, "remote should win when local has no timestamp");
    assert_eq!(result["theme"], "light");
    assert_eq!(result[TS_KEY], 1_000_000i64);
}

#[test]
fn test_local_empty_remote_wins() {
    let local = map(json!({}));
    let remote = map(json!({ "fontSize": 14, TS_KEY: 1_000_000i64 }));
    let (result, local_wins) = merge_settings_json(&local, &remote, SystemTime::now());

    assert!(!local_wins);
    assert_eq!(result["fontSize"], 14);
}

// ---------------------------------------------------------------------------
// Rule 3: both have timestamps — compare local mtime vs remote timestamp
// ---------------------------------------------------------------------------

#[test]
fn test_local_newer_wins() {
    let local_mtime = sys_time_ms(2_000_000);
    let local = map(json!({ "theme": "dark", TS_KEY: 500_000i64 }));
    let remote = map(json!({ "theme": "light", TS_KEY: 1_000_000i64 }));
    let (result, local_wins) = merge_settings_json(&local, &remote, local_mtime);

    assert!(local_wins, "local should win when file mtime is newer");
    assert_eq!(result["theme"], "dark");
    // lastModifiedTimestamp should be updated to match file mtime
    assert_eq!(result[TS_KEY], 2_000_000i64);
}

#[test]
fn test_remote_newer_wins() {
    let local_mtime = sys_time_ms(1_000_000);
    let local = map(json!({ "theme": "dark", TS_KEY: 500_000i64 }));
    let remote = map(json!({ "theme": "light", TS_KEY: 2_000_000i64 }));
    let (result, local_wins) = merge_settings_json(&local, &remote, local_mtime);

    assert!(!local_wins, "remote should win when its timestamp is newer");
    assert_eq!(result["theme"], "light");
    assert_eq!(result[TS_KEY], 2_000_000i64);
}

#[test]
fn test_equal_timestamps_local_wins() {
    let ts: i64 = 1_000_000;
    let local_mtime = sys_time_ms(ts);
    let local = map(json!({ "theme": "dark", TS_KEY: 500_000i64 }));
    let remote = map(json!({ "theme": "light", TS_KEY: ts }));
    let (result, local_wins) = merge_settings_json(&local, &remote, local_mtime);

    assert!(local_wins, "local should win when timestamps are equal");
    assert_eq!(result["theme"], "dark");
}

// ---------------------------------------------------------------------------
// Edge case: remote has no lastModifiedTimestamp
// ---------------------------------------------------------------------------

#[test]
fn test_remote_no_timestamp_local_wins() {
    let local_mtime = sys_time_ms(1_000_000);
    let local = map(json!({ "theme": "dark", TS_KEY: 500_000i64 }));
    let remote = map(json!({ "theme": "light" })); // no TS_KEY
    let (result, local_wins) = merge_settings_json(&local, &remote, local_mtime);

    assert!(local_wins, "local should win when remote has no timestamp");
    assert_eq!(result["theme"], "dark");
    assert_eq!(result[TS_KEY], 1_000_000i64);
}

// ---------------------------------------------------------------------------
// Whole-file semantics: the winner's content is used entirely
// ---------------------------------------------------------------------------

#[test]
fn test_remote_wins_uses_entire_remote_content() {
    let local = map(json!({ "theme": "dark", "extraLocalKey": true }));
    let remote = map(json!({ "theme": "light", "fontSize": 14, TS_KEY: 2_000_000i64 }));
    let (result, local_wins) = merge_settings_json(&local, &remote, sys_time_ms(1_000_000));

    assert!(!local_wins);
    // Remote content is used as-is, local-only keys are NOT preserved
    assert_eq!(result["theme"], "light");
    assert_eq!(result["fontSize"], 14);
    assert!(!result.contains_key("extraLocalKey"));
}

#[test]
fn test_local_wins_uses_entire_local_content() {
    let local = map(json!({ "theme": "dark", "extraLocalKey": true, TS_KEY: 500_000i64 }));
    let remote = map(json!({ "theme": "light", "fontSize": 14, TS_KEY: 1_000_000i64 }));
    let (result, local_wins) = merge_settings_json(&local, &remote, sys_time_ms(2_000_000));

    assert!(local_wins);
    // Local content is used, remote-only keys are NOT preserved
    assert_eq!(result["theme"], "dark");
    assert!(result["extraLocalKey"].as_bool().unwrap());
    assert!(!result.contains_key("fontSize"));
}

// ---------------------------------------------------------------------------
// local_wins flag (controls whether remote should be updated)
// ---------------------------------------------------------------------------

#[test]
fn test_local_wins_flag_true_means_update_remote() {
    let local = map(json!({ "theme": "dark", TS_KEY: 500_000i64 }));
    let remote = map(json!({ "theme": "dark", TS_KEY: 1_000_000i64 }));
    let (_, local_wins) = merge_settings_json(&local, &remote, sys_time_ms(2_000_000));

    assert!(local_wins, "local_wins=true means caller should update remote");
}

#[test]
fn test_local_wins_flag_false_means_no_remote_update() {
    let local = map(json!({ "theme": "dark", TS_KEY: 500_000i64 }));
    let remote = map(json!({ "theme": "light", TS_KEY: 2_000_000i64 }));
    let (_, local_wins) = merge_settings_json(&local, &remote, sys_time_ms(1_000_000));

    assert!(!local_wins, "local_wins=false means remote is already correct");
}
