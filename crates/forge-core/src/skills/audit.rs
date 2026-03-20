use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Audit record for skill operations (schema defined in v1, populated in v2+)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditRecord {
    pub timestamp: DateTime<Utc>,
    pub skill_name: String,
    pub skill_version: String,
    pub skill_path: PathBuf,
    pub action: String,
    pub inputs: HashMap<String, String>,
    pub permissions_requested: PermissionsRecord,
    pub user_approved: bool,
    pub exit_code: Option<i32>,
    pub duration_ms: u64,
    pub files_modified: Vec<PathBuf>,
    pub invocation_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionsRecord {
    pub read_files: bool,
    pub write_files: bool,
    pub network: bool,
    pub execute_commands: bool,
}

/// Placeholder: audit log reading will be implemented when execution is added (v2+)
pub fn read_audit_log(_path: &Path) -> Vec<AuditRecord> {
    // No audit entries exist in v1 (metadata-only)
    Vec::new()
}
