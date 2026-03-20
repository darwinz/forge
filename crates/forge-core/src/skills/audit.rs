//! Audit log reading and filtering for skill execution records.
//!
//! Records are JSONL (one JSON object per line) written by the execution engine.
//! This module reads, parses, filters, and formats those records.
//! Malformed lines are skipped with a warning, not fatal.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use crate::skills::execution::redact_value;

/// An audit record as written by the execution engine.
/// Fields are optional to handle schema evolution gracefully.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditRecord {
    #[serde(default)]
    pub invocation_id: String,
    #[serde(default)]
    pub timestamp: String,
    #[serde(default)]
    pub skill_name: String,
    #[serde(default)]
    pub skill_version: String,
    #[serde(default)]
    pub skill_source: String,
    #[serde(default)]
    pub skill_content_hash: String,
    #[serde(default)]
    pub action: String,
    #[serde(default)]
    pub inputs: HashMap<String, String>,
    #[serde(default)]
    pub user_approved: bool,
    #[serde(default)]
    pub files_written: Vec<String>,
    #[serde(default)]
    pub duration_ms: u64,
    #[serde(default)]
    pub error: Option<String>,
}

/// Read audit records from a JSONL file.
/// Malformed lines are skipped. Returns records in file order.
pub fn read_audit_log(path: &Path) -> Vec<AuditRecord> {
    if !path.exists() {
        return Vec::new();
    }

    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|line| match serde_json::from_str::<AuditRecord>(line) {
            Ok(record) => Some(record),
            Err(e) => {
                tracing::warn!("Skipping malformed audit record: {e}");
                None
            }
        })
        .collect()
}

/// Filter audit records by skill name.
pub fn filter_by_skill(records: &[AuditRecord], skill: &str) -> Vec<AuditRecord> {
    records
        .iter()
        .filter(|r| r.skill_name == skill)
        .cloned()
        .collect()
}

/// Filter audit records by action.
pub fn filter_by_action(records: &[AuditRecord], action: &str) -> Vec<AuditRecord> {
    records
        .iter()
        .filter(|r| r.action == action)
        .cloned()
        .collect()
}

/// Take the last N records.
pub fn last_n(records: &[AuditRecord], n: usize) -> Vec<AuditRecord> {
    if records.len() <= n {
        records.to_vec()
    } else {
        records[records.len() - n..].to_vec()
    }
}

/// Format a single audit record for human display.
/// Applies redaction to inputs.
pub fn format_record(record: &AuditRecord) -> String {
    let mut lines = Vec::new();

    // Header line
    let status_marker = match record.action.as_str() {
        "run" => "✓",
        "dry-run" => "~",
        "rejected" | "denied" => "✗",
        _ => "?",
    };

    lines.push(format!(
        "  {status_marker} {} v{} [{}] — {} ({}ms)",
        record.skill_name,
        record.skill_version,
        record.action,
        format_timestamp(&record.timestamp),
        record.duration_ms,
    ));

    // Source and hash
    if !record.skill_source.is_empty() {
        lines.push(format!("    source: {}", record.skill_source));
    }

    // Inputs (redacted)
    if !record.inputs.is_empty() {
        let redacted: Vec<String> = record
            .inputs
            .iter()
            .map(|(k, v)| format!("{k}={}", redact_value(k, v)))
            .collect();
        lines.push(format!("    inputs: {}", redacted.join(", ")));
    }

    // Files written
    if !record.files_written.is_empty() {
        lines.push(format!("    files: {}", record.files_written.join(", ")));
    }

    // Error
    if let Some(ref err) = record.error {
        lines.push(format!("    error: {err}"));
    }

    lines.join("\n")
}

/// Format a timestamp for display (truncate to seconds).
fn format_timestamp(ts: &str) -> &str {
    // ISO 8601 timestamps — show date+time, truncate fractional seconds
    if ts.len() > 19 {
        &ts[..19]
    } else {
        ts
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_record(action: &str, skill: &str) -> AuditRecord {
        AuditRecord {
            invocation_id: "test-id".into(),
            timestamp: "2026-03-20T15:30:00Z".into(),
            skill_name: skill.into(),
            skill_version: "1.0.0".into(),
            skill_source: "user".into(),
            skill_content_hash: "sha256:abc".into(),
            action: action.into(),
            inputs: HashMap::from([("name".into(), "users".into())]),
            user_approved: true,
            files_written: vec!["src/route.ts".into()],
            duration_ms: 42,
            error: None,
        }
    }

    #[test]
    fn test_read_audit_log_nonexistent() {
        let records = read_audit_log(Path::new("/tmp/forge_nonexistent_audit.jsonl"));
        assert!(records.is_empty());
    }

    #[test]
    fn test_read_audit_log_valid_jsonl() {
        let dir = std::env::temp_dir().join("forge_test_audit_read");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("audit.jsonl");

        let record = sample_record("run", "scaffold");
        let json = serde_json::to_string(&record).unwrap();
        std::fs::write(&path, format!("{json}\n")).unwrap();

        let records = read_audit_log(&path);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].skill_name, "scaffold");
        assert_eq!(records[0].action, "run");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_read_audit_log_skips_malformed() {
        let dir = std::env::temp_dir().join("forge_test_audit_malformed");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("audit.jsonl");

        let record = sample_record("run", "good-skill");
        let json = serde_json::to_string(&record).unwrap();
        std::fs::write(&path, format!("{json}\nnot-json-at-all\n{json}\n")).unwrap();

        let records = read_audit_log(&path);
        // Malformed line skipped, two valid records parsed
        assert_eq!(records.len(), 2);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_filter_by_skill() {
        let records = vec![
            sample_record("run", "alpha"),
            sample_record("run", "beta"),
            sample_record("dry-run", "alpha"),
        ];
        let filtered = filter_by_skill(&records, "alpha");
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_filter_by_action() {
        let records = vec![
            sample_record("run", "alpha"),
            sample_record("rejected", "alpha"),
            sample_record("run", "beta"),
        ];
        let filtered = filter_by_action(&records, "rejected");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].action, "rejected");
    }

    #[test]
    fn test_last_n() {
        let records = vec![
            sample_record("run", "a"),
            sample_record("run", "b"),
            sample_record("run", "c"),
        ];
        let last_2 = last_n(&records, 2);
        assert_eq!(last_2.len(), 2);
        assert_eq!(last_2[0].skill_name, "b");
        assert_eq!(last_2[1].skill_name, "c");
    }

    #[test]
    fn test_last_n_larger_than_records() {
        let records = vec![sample_record("run", "a")];
        let result = last_n(&records, 100);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_format_record_run() {
        let record = sample_record("run", "scaffold");
        let output = format_record(&record);
        assert!(output.contains("✓"));
        assert!(output.contains("scaffold"));
        assert!(output.contains("[run]"));
        assert!(output.contains("42ms"));
    }

    #[test]
    fn test_format_record_rejected() {
        let mut record = sample_record("rejected", "bad-skill");
        record.error = Some("execute_commands not supported".into());
        let output = format_record(&record);
        assert!(output.contains("✗"));
        assert!(output.contains("rejected"));
        assert!(output.contains("execute_commands"));
    }

    #[test]
    fn test_format_record_redacts_secrets() {
        let mut record = sample_record("run", "scaffold");
        record
            .inputs
            .insert("api_key".into(), "sk-secret-123".into());
        let output = format_record(&record);
        assert!(output.contains("[redacted]"));
        assert!(!output.contains("sk-secret-123"));
    }
}
