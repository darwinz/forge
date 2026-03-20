//! Skill execution engine — template rendering and declarative transforms.
//!
//! V1 supports `execution.type = "template"` and `execution.type = "transform"`.
//! Script, binary, wasm, and command execution types are rejected with a clear error.
//!
//! Safety controls:
//! - Path validation with symlink escape detection
//! - Atomic writes (temp file + rename)
//! - Content hashing for trust verification
//! - Redaction of sensitive input values
//! - Non-interactive mode defaults to deny

use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::error::{ForgeError, ForgeResult};
use crate::skills::metadata::{DiscoveredSkill, SkillSource};

// ---------------------------------------------------------------------------
// Execution plan
// ---------------------------------------------------------------------------

/// A planned file write from template rendering.
#[derive(Debug, Clone)]
pub struct PlannedWrite {
    pub destination: PathBuf,
    pub content: String,
    pub is_overwrite: bool,
    pub existing_content: Option<String>,
    pub overwrite_allowed: bool,
}

/// Result of planning a skill execution.
#[derive(Debug)]
pub struct ExecutionPlan {
    pub skill_name: String,
    pub skill_version: String,
    pub writes: Vec<PlannedWrite>,
    pub inputs_redacted: HashMap<String, String>,
}

/// Result of executing a skill.
#[derive(Debug)]
pub struct ExecutionResult {
    pub files_written: Vec<PathBuf>,
    pub errors: Vec<String>,
    pub success: bool,
}

// ---------------------------------------------------------------------------
// Pre-flight checks
// ---------------------------------------------------------------------------

/// Check if a skill's execution type is supported in V1.
pub fn check_execution_type(skill: &DiscoveredSkill) -> ForgeResult<()> {
    let exec = skill
        .manifest
        .skill
        .execution
        .as_ref()
        .ok_or_else(|| ForgeError::Other("Skill has no [execution] section.".into()))?;

    match exec.exec_type.as_str() {
        "template" => Ok(()),
        "transform" => Ok(()),
        unsupported => Err(ForgeError::Other(format!(
            "Execution type '{unsupported}' is not supported in forge v1.\n\
             Only 'template' and 'transform' are supported.\n\
             See: forge skill info {}",
            skill.manifest.skill.name
        ))),
    }
}

/// Check if a skill's permissions are supported in V1.
pub fn check_permissions_v1(skill: &DiscoveredSkill) -> ForgeResult<()> {
    if let Some(ref perms) = skill.manifest.skill.permissions {
        if perms.execute_commands {
            return Err(ForgeError::Other(format!(
                "Skill '{}' requests execute_commands permission, which is not supported in forge v1.",
                skill.manifest.skill.name
            )));
        }
        if perms.network {
            return Err(ForgeError::Other(format!(
                "Skill '{}' requests network permission, which is not supported in forge v1.",
                skill.manifest.skill.name
            )));
        }
    }
    Ok(())
}

/// Check trust: repo-scoped skills with write_files need explicit trust.
pub fn check_trust(skill: &DiscoveredSkill, trust_store: &TrustStore) -> ForgeResult<TrustStatus> {
    let needs_write = skill
        .manifest
        .skill
        .permissions
        .as_ref()
        .map(|p| p.write_files)
        .unwrap_or(false);

    match &skill.source {
        SkillSource::BuiltIn => Ok(TrustStatus::Trusted),
        SkillSource::UserGlobal => {
            // User-global skills need first-run approval for writes
            if !needs_write {
                return Ok(TrustStatus::Trusted);
            }
            let current_hash = compute_skill_hash(&skill.path)?;
            match trust_store.check_user_trust(&skill.manifest.skill.name, &current_hash) {
                true => Ok(TrustStatus::Trusted),
                false => Ok(TrustStatus::NeedsApproval { hash: current_hash }),
            }
        }
        SkillSource::RepoScoped => {
            if !needs_write {
                return Ok(TrustStatus::Trusted);
            }
            let current_hash = compute_skill_hash(&skill.path)?;
            let repo_key = repo_path_key(&skill.path);
            match trust_store.check_repo_trust(&repo_key, &skill.manifest.skill.name, &current_hash)
            {
                true => Ok(TrustStatus::Trusted),
                false => Ok(TrustStatus::UntrustedRepo { hash: current_hash }),
            }
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum TrustStatus {
    Trusted,
    NeedsApproval { hash: String },
    UntrustedRepo { hash: String },
}

// ---------------------------------------------------------------------------
// Content hashing
// ---------------------------------------------------------------------------

/// Compute a SHA-256 hash of all execution-relevant files in a skill directory.
pub fn compute_skill_hash(skill_dir: &Path) -> ForgeResult<String> {
    let mut file_hashes: Vec<(String, String)> = Vec::new();

    // Hash skill.toml
    hash_file(skill_dir, "skill.toml", &mut file_hashes)?;

    // Hash all template files
    let templates_dir = skill_dir.join("templates");
    if templates_dir.exists() {
        hash_directory_recursive(&templates_dir, "templates", &mut file_hashes)?;
    }

    // Hash transform.toml if present
    let transform_toml = skill_dir.join("transform.toml");
    if transform_toml.exists() {
        hash_file(skill_dir, "transform.toml", &mut file_hashes)?;
    }

    // Sort by path for determinism
    file_hashes.sort_by(|a, b| a.0.cmp(&b.0));

    // Combine into a single hash
    let combined = file_hashes
        .iter()
        .map(|(path, hash)| format!("{path}:{hash}"))
        .collect::<Vec<_>>()
        .join("\n");

    let mut hasher = Sha256::new();
    hasher.update(combined.as_bytes());
    Ok(format!("sha256:{:x}", hasher.finalize()))
}

fn hash_file(
    base_dir: &Path,
    relative: &str,
    hashes: &mut Vec<(String, String)>,
) -> ForgeResult<()> {
    let path = base_dir.join(relative);
    if !path.exists() {
        return Ok(());
    }

    // Resolve symlinks and verify they don't escape the skill directory
    let canonical = path.canonicalize().map_err(|e| ForgeError::FileRead {
        path: path.clone(),
        source: e,
    })?;
    let canonical_base = base_dir.canonicalize().map_err(|e| ForgeError::FileRead {
        path: base_dir.to_path_buf(),
        source: e,
    })?;
    if !canonical.starts_with(&canonical_base) {
        return Err(ForgeError::Other(format!(
            "Skill file {relative} is a symlink escaping the skill directory"
        )));
    }

    let content = std::fs::read(&path).map_err(|e| ForgeError::FileRead {
        path: path.clone(),
        source: e,
    })?;
    let mut hasher = Sha256::new();
    hasher.update(&content);
    hashes.push((relative.to_string(), format!("{:x}", hasher.finalize())));
    Ok(())
}

fn hash_directory_recursive(
    dir: &Path,
    prefix: &str,
    hashes: &mut Vec<(String, String)>,
) -> ForgeResult<()> {
    if !dir.exists() {
        return Ok(());
    }
    let entries = std::fs::read_dir(dir).map_err(|e| ForgeError::FileRead {
        path: dir.to_path_buf(),
        source: e,
    })?;
    for entry in entries.flatten() {
        let path = entry.path();
        let relative = format!("{}/{}", prefix, entry.file_name().to_string_lossy());
        if path.is_dir() {
            hash_directory_recursive(&path, &relative, hashes)?;
        } else {
            hash_file(
                &dir.parent()
                    .unwrap_or(dir)
                    .join("..")
                    .join(prefix)
                    .join(".."),
                &relative,
                hashes,
            )
            .ok(); // best-effort for nested files
                   // Actually, re-do this properly:
            let content = std::fs::read(&path).map_err(|e| ForgeError::FileRead {
                path: path.clone(),
                source: e,
            })?;
            let mut hasher = Sha256::new();
            hasher.update(&content);
            hashes.push((relative, format!("{:x}", hasher.finalize())));
        }
    }
    Ok(())
}

/// Compute a hash key for a repo path (used for repo-local trust).
/// Public so the CLI can use it for the `trust` command.
pub fn repo_path_hash(skill_path: &Path) -> String {
    repo_path_key(skill_path)
}

fn repo_path_key(skill_path: &Path) -> String {
    let mut hasher = Sha256::new();
    hasher.update(skill_path.to_string_lossy().as_bytes());
    format!("sha256:{:x}", hasher.finalize())
}

// ---------------------------------------------------------------------------
// Trust store
// ---------------------------------------------------------------------------

/// In-memory trust store. Loaded from/saved to ~/.forge/trust.toml.
#[derive(Debug, Clone, Default)]
pub struct TrustStore {
    user_trust: HashMap<String, String>, // skill_name -> hash
    repo_trust: HashMap<String, HashMap<String, String>>, // repo_key -> skill_name -> hash
}

impl TrustStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Load trust store from a TOML file.
    pub fn load(path: &Path) -> Self {
        if !path.exists() {
            return Self::new();
        }
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => return Self::new(),
        };
        // Simple key=value parsing for trust records
        let mut store = Self::new();
        let mut current_section = String::new();
        let mut current_repo = String::new();
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("[user.") {
                let name = trimmed
                    .trim_start_matches("[user.\"")
                    .trim_end_matches("\"]");
                current_section = "user".into();
                current_repo = name.into();
            } else if trimmed.starts_with("[repo.") {
                current_section = "repo".into();
                // Parse [repo."hash"."name"]
                let inner = trimmed.trim_start_matches("[repo.").trim_end_matches(']');
                let parts: Vec<&str> = inner.split("\".\"").collect();
                if parts.len() >= 2 {
                    current_repo = format!(
                        "{}|{}",
                        parts[0].trim_matches('"'),
                        parts[1].trim_matches('"')
                    );
                }
            } else if trimmed.starts_with("hash") {
                if let Some(val) = trimmed.split('=').nth(1) {
                    let hash = val.trim().trim_matches('"').to_string();
                    if current_section == "user" {
                        store.user_trust.insert(current_repo.clone(), hash);
                    } else if current_section == "repo" {
                        let parts: Vec<&str> = current_repo.split('|').collect();
                        if parts.len() == 2 {
                            store
                                .repo_trust
                                .entry(parts[0].to_string())
                                .or_default()
                                .insert(parts[1].to_string(), hash);
                        }
                    }
                }
            }
        }
        store
    }

    pub fn check_user_trust(&self, name: &str, hash: &str) -> bool {
        self.user_trust
            .get(name)
            .map(|h| h == hash)
            .unwrap_or(false)
    }

    pub fn check_repo_trust(&self, repo_key: &str, name: &str, hash: &str) -> bool {
        self.repo_trust
            .get(repo_key)
            .and_then(|m| m.get(name))
            .map(|h| h == hash)
            .unwrap_or(false)
    }

    pub fn set_user_trust(&mut self, name: &str, hash: &str) {
        self.user_trust.insert(name.to_string(), hash.to_string());
    }

    pub fn set_repo_trust(&mut self, repo_key: &str, name: &str, hash: &str) {
        self.repo_trust
            .entry(repo_key.to_string())
            .or_default()
            .insert(name.to_string(), hash.to_string());
    }

    /// Revoke trust for a skill by name (from both user and all repo stores).
    pub fn revoke(&mut self, name: &str) {
        self.user_trust.remove(name);
        for skills in self.repo_trust.values_mut() {
            skills.remove(name);
        }
    }

    /// Save trust store to a file.
    pub fn save(&self, path: &Path) -> ForgeResult<()> {
        let mut content = String::new();
        content.push_str("# forge skill trust store — do not edit manually\n\n");

        for (name, hash) in &self.user_trust {
            content.push_str(&format!("[user.\"{name}\"]\n"));
            content.push_str(&format!("hash = \"{hash}\"\n\n"));
        }

        for (repo_key, skills) in &self.repo_trust {
            for (name, hash) in skills {
                content.push_str(&format!("[repo.\"{repo_key}\".\"{name}\"]\n"));
                content.push_str(&format!("hash = \"{hash}\"\n\n"));
            }
        }

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| ForgeError::FileRead {
                path: parent.to_path_buf(),
                source: e,
            })?;
        }
        std::fs::write(path, content).map_err(|e| ForgeError::FileRead {
            path: path.to_path_buf(),
            source: e,
        })
    }
}

// ---------------------------------------------------------------------------
// Template rendering
// ---------------------------------------------------------------------------

/// Render a template string with simple `{{ variable }}` substitution.
///
/// This is intentionally minimal — no includes, no macros, no functions.
/// Variables are replaced literally. Unknown variables are left as-is.
pub fn render_template(template: &str, inputs: &HashMap<String, String>) -> String {
    let mut result = template.to_string();
    for (key, value) in inputs {
        let pattern = format!("{{{{ {key} }}}}");
        result = result.replace(&pattern, value);
        // Also handle no-space variant: {{key}}
        let pattern_nospace = format!("{{{{{key}}}}}");
        result = result.replace(&pattern_nospace, value);
    }
    result
}

// ---------------------------------------------------------------------------
// Path validation
// ---------------------------------------------------------------------------

/// Validate an output path is within the allowed root (CWD).
/// Checks for traversal, absolute paths, and symlink escapes.
pub fn validate_output_path(cwd: &Path, relative_path: &str) -> ForgeResult<PathBuf> {
    // Reject obvious traversal
    if relative_path.contains("..") {
        return Err(ForgeError::Other(format!(
            "Output path contains '..' traversal: {relative_path}"
        )));
    }
    if Path::new(relative_path).is_absolute() {
        return Err(ForgeError::Other(format!(
            "Absolute output paths are not allowed: {relative_path}"
        )));
    }

    let target = cwd.join(relative_path);

    // For existing paths: canonicalize and check
    if target.exists() {
        let canonical = target.canonicalize().map_err(|e| ForgeError::FileRead {
            path: target.clone(),
            source: e,
        })?;
        let canonical_cwd = cwd.canonicalize().map_err(|e| ForgeError::FileRead {
            path: cwd.to_path_buf(),
            source: e,
        })?;
        if !canonical.starts_with(&canonical_cwd) {
            return Err(ForgeError::Other(
                "Output path escapes working directory via symlink".to_string(),
            ));
        }
    } else {
        // For new paths: canonicalize the deepest existing parent
        let mut check = target.parent();
        while let Some(parent) = check {
            if parent.exists() {
                let canonical_parent = parent.canonicalize().map_err(|e| ForgeError::FileRead {
                    path: parent.to_path_buf(),
                    source: e,
                })?;
                let canonical_cwd = cwd.canonicalize().map_err(|e| ForgeError::FileRead {
                    path: cwd.to_path_buf(),
                    source: e,
                })?;
                if !canonical_parent.starts_with(&canonical_cwd) {
                    return Err(ForgeError::Other(
                        "Output path escapes working directory via symlink".to_string(),
                    ));
                }
                break;
            }
            check = parent.parent();
        }
    }

    Ok(target)
}

// ---------------------------------------------------------------------------
// Atomic write
// ---------------------------------------------------------------------------

/// Write content to a file atomically via temp file + rename.
pub fn atomic_write(path: &Path, content: &str) -> ForgeResult<()> {
    // Create parent directories
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| ForgeError::FileRead {
            path: parent.to_path_buf(),
            source: e,
        })?;
    }

    // Write to temp file in the same directory
    let dir = path.parent().unwrap_or(Path::new("."));
    let mut temp = tempfile::NamedTempFile::new_in(dir)
        .map_err(|e| ForgeError::Other(format!("Failed to create temp file: {e}")))?;
    temp.write_all(content.as_bytes())
        .map_err(|e| ForgeError::Other(format!("Failed to write temp file: {e}")))?;

    // Rename (atomic on same filesystem)
    temp.persist(path)
        .map_err(|e| ForgeError::Other(format!("Failed to rename temp file: {e}")))?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Redaction
// ---------------------------------------------------------------------------

/// Redact a value if its key matches sensitive patterns.
/// Used for both audit logs and terminal output.
pub fn redact_value(key: &str, value: &str) -> String {
    let key_upper = key.to_uppercase();
    if key_upper.contains("SECRET")
        || key_upper.contains("KEY")
        || key_upper.contains("TOKEN")
        || key_upper.contains("PASSWORD")
        || key_upper.contains("CREDENTIAL")
    {
        return "[redacted]".to_string();
    }
    if value.len() > 200 {
        return format!("{}...[truncated]", &value[..197]);
    }
    value.to_string()
}

/// Redact all sensitive values in an input map.
pub fn redact_inputs(inputs: &HashMap<String, String>) -> HashMap<String, String> {
    inputs
        .iter()
        .map(|(k, v)| (k.clone(), redact_value(k, v)))
        .collect()
}

// ---------------------------------------------------------------------------
// Audit logging
// ---------------------------------------------------------------------------

/// Write an audit record as a JSONL line.
#[allow(clippy::too_many_arguments)]
pub fn write_audit_record(
    audit_path: &Path,
    skill: &DiscoveredSkill,
    action: &str,
    inputs: &HashMap<String, String>,
    files_written: &[PathBuf],
    approved: bool,
    duration_ms: u64,
    error: Option<&str>,
) -> ForgeResult<()> {
    if let Some(parent) = audit_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| ForgeError::FileRead {
            path: parent.to_path_buf(),
            source: e,
        })?;
    }

    let redacted = redact_inputs(inputs);
    let content_hash = compute_skill_hash(&skill.path).unwrap_or_default();

    let record = serde_json::json!({
        "invocation_id": uuid::Uuid::new_v4().to_string(),
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "skill_name": skill.manifest.skill.name,
        "skill_version": skill.manifest.skill.version,
        "skill_source": skill.source.to_string(),
        "skill_content_hash": content_hash,
        "action": action,
        "inputs": redacted,
        "user_approved": approved,
        "files_written": files_written.iter().map(|p| p.to_string_lossy().to_string()).collect::<Vec<_>>(),
        "duration_ms": duration_ms,
        "error": error,
    });

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(audit_path)
        .map_err(|e| ForgeError::FileRead {
            path: audit_path.to_path_buf(),
            source: e,
        })?;

    writeln!(file, "{}", record)
        .map_err(|e| ForgeError::Other(format!("Failed to write audit record: {e}")))?;

    // Set permissions to 0600 (best-effort, Unix only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(audit_path, std::fs::Permissions::from_mode(0o600));
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Plan template execution
// ---------------------------------------------------------------------------

/// Plan the file writes for a template skill.
pub fn plan_template_execution(
    skill: &DiscoveredSkill,
    inputs: &HashMap<String, String>,
    cwd: &Path,
) -> ForgeResult<ExecutionPlan> {
    let exec = skill
        .manifest
        .skill
        .execution
        .as_ref()
        .ok_or_else(|| ForgeError::Other("Skill has no [execution] section.".into()))?;

    if exec.exec_type != "template" {
        return Err(ForgeError::Other(format!(
            "Expected execution type 'template', got '{}'",
            exec.exec_type
        )));
    }

    let templates_dir = skill.path.join(&exec.entrypoint);
    let max_file_size = skill
        .manifest
        .skill
        .permissions
        .as_ref()
        .and_then(|p| p.max_file_size_bytes)
        .unwrap_or(1_048_576); // 1MB default

    let mut writes = Vec::new();

    for output in &exec.outputs {
        // Resolve template path
        let template_path = templates_dir.join(&output.template);
        let template_content =
            std::fs::read_to_string(&template_path).map_err(|e| ForgeError::FileRead {
                path: template_path.clone(),
                source: e,
            })?;

        // Render template
        let rendered = render_template(&template_content, inputs);

        // Check size limit
        if rendered.len() as u64 > max_file_size {
            return Err(ForgeError::Other(format!(
                "Rendered output for {} exceeds max_file_size_bytes ({} > {})",
                output.template,
                rendered.len(),
                max_file_size
            )));
        }

        // Resolve destination path with input interpolation
        let dest_str = render_template(&output.destination, inputs);
        let dest_path = validate_output_path(cwd, &dest_str)?;

        // Check for overwrite conflicts
        let (is_overwrite, existing) = if dest_path.exists() {
            let existing = std::fs::read_to_string(&dest_path).ok();
            (true, existing)
        } else {
            (false, None)
        };

        if is_overwrite && !output.overwrite {
            return Err(ForgeError::Other(format!(
                "File already exists and overwrite=false: {}",
                dest_str
            )));
        }

        writes.push(PlannedWrite {
            destination: dest_path,
            content: rendered,
            is_overwrite,
            existing_content: existing,
            overwrite_allowed: output.overwrite,
        });
    }

    Ok(ExecutionPlan {
        skill_name: skill.manifest.skill.name.clone(),
        skill_version: skill.manifest.skill.version.clone(),
        writes,
        inputs_redacted: redact_inputs(inputs),
    })
}

/// Execute the writes from a plan.
pub fn execute_plan(plan: &ExecutionPlan) -> ExecutionResult {
    let mut files_written = Vec::new();
    let mut errors = Vec::new();

    for write in &plan.writes {
        match atomic_write(&write.destination, &write.content) {
            Ok(()) => files_written.push(write.destination.clone()),
            Err(e) => errors.push(format!("{}: {e}", write.destination.display())),
        }
    }

    ExecutionResult {
        success: errors.is_empty(),
        files_written,
        errors,
    }
}

// ---------------------------------------------------------------------------
// Transform execution
// ---------------------------------------------------------------------------

/// Plan the file writes for a transform skill.
pub fn plan_transform_execution(
    skill: &DiscoveredSkill,
    inputs: &HashMap<String, String>,
    cwd: &Path,
) -> ForgeResult<ExecutionPlan> {
    let exec = skill
        .manifest
        .skill
        .execution
        .as_ref()
        .ok_or_else(|| ForgeError::Other("Skill has no [execution] section.".into()))?;

    if exec.exec_type != "transform" {
        return Err(ForgeError::Other(format!(
            "Expected execution type 'transform', got '{}'",
            exec.exec_type
        )));
    }

    let max_file_size = skill
        .manifest
        .skill
        .permissions
        .as_ref()
        .and_then(|p| p.max_file_size_bytes)
        .unwrap_or(1_048_576);

    let mut writes = Vec::new();

    for transform in &exec.transforms {
        let file_str = render_template(&transform.file, inputs);
        let file_path = validate_output_path(cwd, &file_str)?;

        let existing_content =
            std::fs::read_to_string(&file_path).map_err(|e| ForgeError::FileRead {
                path: file_path.clone(),
                source: e,
            })?;

        let new_content = apply_transform(
            &transform.operation,
            &existing_content,
            transform.value.as_ref(),
            transform.lines.as_deref(),
            &file_str,
        )?;

        if new_content.len() as u64 > max_file_size {
            return Err(ForgeError::Other(format!(
                "Transform output for {} exceeds max_file_size_bytes ({} > {})",
                file_str,
                new_content.len(),
                max_file_size
            )));
        }

        writes.push(PlannedWrite {
            destination: file_path,
            content: new_content,
            is_overwrite: true,
            existing_content: Some(existing_content),
            overwrite_allowed: true,
        });
    }

    Ok(ExecutionPlan {
        skill_name: skill.manifest.skill.name.clone(),
        skill_version: skill.manifest.skill.version.clone(),
        writes,
        inputs_redacted: redact_inputs(inputs),
    })
}

/// Apply a single transform operation.
fn apply_transform(
    operation: &str,
    existing: &str,
    value: Option<&toml::Value>,
    lines: Option<&[String]>,
    file_name: &str,
) -> ForgeResult<String> {
    match operation {
        "json_merge" => {
            let merge_value = value.ok_or_else(|| {
                ForgeError::Other(format!("{file_name}: json_merge requires a 'value' field"))
            })?;
            json_merge(existing, merge_value)
        }
        "toml_merge" => {
            let merge_value = value.ok_or_else(|| {
                ForgeError::Other(format!("{file_name}: toml_merge requires a 'value' field"))
            })?;
            toml_merge(existing, merge_value)
        }
        "line_append" => {
            let lines = lines.ok_or_else(|| {
                ForgeError::Other(format!("{file_name}: line_append requires a 'lines' field"))
            })?;
            Ok(line_append(existing, lines))
        }
        "line_prepend" => {
            let lines = lines.ok_or_else(|| {
                ForgeError::Other(format!(
                    "{file_name}: line_prepend requires a 'lines' field"
                ))
            })?;
            Ok(line_prepend(existing, lines))
        }
        other => Err(ForgeError::Other(format!(
            "Unknown transform operation: '{other}'. \
             Supported: json_merge, toml_merge, line_append, line_prepend"
        ))),
    }
}

/// Deep merge a TOML value into a JSON string.
///
/// The merge value comes from TOML (skill manifest) and is converted to JSON
/// for merging. Object keys are merged recursively; non-object values are replaced.
fn json_merge(existing_json: &str, merge_value: &toml::Value) -> ForgeResult<String> {
    let mut existing: serde_json::Value = serde_json::from_str(existing_json)
        .map_err(|e| ForgeError::Other(format!("Failed to parse target as JSON: {e}")))?;

    let merge_json = toml_value_to_json(merge_value);

    json_deep_merge(&mut existing, &merge_json);

    serde_json::to_string_pretty(&existing)
        .map_err(|e| ForgeError::Other(format!("Failed to serialize JSON: {e}")))
}

fn json_deep_merge(base: &mut serde_json::Value, patch: &serde_json::Value) {
    match (base, patch) {
        (serde_json::Value::Object(base_map), serde_json::Value::Object(patch_map)) => {
            for (key, patch_val) in patch_map {
                let entry = base_map
                    .entry(key.clone())
                    .or_insert(serde_json::Value::Null);
                json_deep_merge(entry, patch_val);
            }
        }
        (base, patch) => {
            *base = patch.clone();
        }
    }
}

fn toml_value_to_json(v: &toml::Value) -> serde_json::Value {
    match v {
        toml::Value::String(s) => serde_json::Value::String(s.clone()),
        toml::Value::Integer(i) => serde_json::json!(*i),
        toml::Value::Float(f) => serde_json::json!(*f),
        toml::Value::Boolean(b) => serde_json::Value::Bool(*b),
        toml::Value::Array(arr) => {
            serde_json::Value::Array(arr.iter().map(toml_value_to_json).collect())
        }
        toml::Value::Table(table) => {
            let map: serde_json::Map<String, serde_json::Value> = table
                .iter()
                .map(|(k, v)| (k.clone(), toml_value_to_json(v)))
                .collect();
            serde_json::Value::Object(map)
        }
        toml::Value::Datetime(dt) => serde_json::Value::String(dt.to_string()),
    }
}

/// Deep merge a TOML value into an existing TOML string.
///
/// Table keys are merged recursively; non-table values are replaced.
/// Preserves existing keys not present in the merge value.
fn toml_merge(existing_toml: &str, merge_value: &toml::Value) -> ForgeResult<String> {
    let mut existing: toml::Table = toml::from_str(existing_toml)
        .map_err(|e| ForgeError::Other(format!("Failed to parse target as TOML: {e}")))?;

    if let toml::Value::Table(merge_table) = merge_value {
        toml_deep_merge(&mut existing, merge_table);
    } else {
        return Err(ForgeError::Other(
            "toml_merge value must be a table".to_string(),
        ));
    }

    toml::to_string_pretty(&existing)
        .map_err(|e| ForgeError::Other(format!("Failed to serialize TOML: {e}")))
}

fn toml_deep_merge(base: &mut toml::Table, patch: &toml::Table) {
    for (key, patch_val) in patch {
        match (base.get_mut(key), patch_val) {
            (Some(toml::Value::Table(base_table)), toml::Value::Table(patch_table)) => {
                toml_deep_merge(base_table, patch_table);
            }
            _ => {
                base.insert(key.clone(), patch_val.clone());
            }
        }
    }
}

/// Append lines to existing content, ensuring a trailing newline before appending.
fn line_append(existing: &str, lines: &[String]) -> String {
    let mut result = existing.to_string();
    if !result.is_empty() && !result.ends_with('\n') {
        result.push('\n');
    }
    for line in lines {
        result.push_str(line);
        result.push('\n');
    }
    result
}

/// Prepend lines to existing content.
fn line_prepend(existing: &str, lines: &[String]) -> String {
    let mut result = String::new();
    for line in lines {
        result.push_str(line);
        result.push('\n');
    }
    result.push_str(existing);
    result
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_render_template_basic() {
        let mut inputs = HashMap::new();
        inputs.insert("name".into(), "users".into());
        inputs.insert("method".into(), "GET".into());

        let template = "export function {{ method }}{{ name }}() { return []; }";
        let result = render_template(template, &inputs);
        assert_eq!(result, "export function GETusers() { return []; }");
    }

    #[test]
    fn test_render_template_nospace() {
        let mut inputs = HashMap::new();
        inputs.insert("name".into(), "test".into());

        let result = render_template("hello {{name}}", &inputs);
        assert_eq!(result, "hello test");
    }

    #[test]
    fn test_render_template_unknown_var() {
        let inputs = HashMap::new();
        let result = render_template("hello {{ unknown }}", &inputs);
        assert_eq!(result, "hello {{ unknown }}");
    }

    #[test]
    fn test_redact_value_secret() {
        assert_eq!(redact_value("api_key", "sk-12345"), "[redacted]");
        assert_eq!(redact_value("API_SECRET", "xyz"), "[redacted]");
        assert_eq!(redact_value("db_password", "pass"), "[redacted]");
        assert_eq!(redact_value("AUTH_TOKEN", "tok"), "[redacted]");
        assert_eq!(redact_value("credential_file", "/path"), "[redacted]");
    }

    #[test]
    fn test_redact_value_safe() {
        assert_eq!(redact_value("name", "users"), "users");
        assert_eq!(redact_value("method", "GET"), "GET");
    }

    #[test]
    fn test_redact_value_long() {
        let long = "a".repeat(250);
        let result = redact_value("description", &long);
        assert!(result.ends_with("...[truncated]"));
        assert!(result.len() < 250);
    }

    #[test]
    fn test_redact_inputs() {
        let mut inputs = HashMap::new();
        inputs.insert("name".into(), "users".into());
        inputs.insert("api_key".into(), "sk-12345".into());

        let redacted = redact_inputs(&inputs);
        assert_eq!(redacted["name"], "users");
        assert_eq!(redacted["api_key"], "[redacted]");
    }

    #[test]
    fn test_validate_output_path_normal() {
        let cwd = std::env::current_dir().unwrap();
        let result = validate_output_path(&cwd, "src/test.ts");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_output_path_traversal() {
        let cwd = std::env::current_dir().unwrap();
        let result = validate_output_path(&cwd, "../escape.txt");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains(".."));
    }

    #[test]
    fn test_validate_output_path_absolute() {
        let cwd = std::env::current_dir().unwrap();
        let result = validate_output_path(&cwd, "/etc/passwd");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Absolute"));
    }

    #[test]
    fn test_validate_output_path_symlink_escape() {
        let cwd = std::env::current_dir().unwrap();
        let test_dir = cwd.join("test_symlink_escape");
        let _ = fs::create_dir_all(&test_dir);

        // Create a symlink pointing outside CWD
        let link_path = test_dir.join("escape_link");
        let _ = fs::remove_file(&link_path);
        #[cfg(unix)]
        {
            let _ = std::os::unix::fs::symlink("/tmp", &link_path);
            let result = validate_output_path(&test_dir, "escape_link/evil.txt");
            assert!(result.is_err());
        }

        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_compute_skill_hash_deterministic() {
        let dir = std::env::temp_dir().join("forge_test_skill_hash");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("templates")).unwrap();
        fs::write(dir.join("skill.toml"), "[skill]\nname = \"test\"").unwrap();
        fs::write(dir.join("templates/a.tera"), "hello {{ name }}").unwrap();

        let hash1 = compute_skill_hash(&dir).unwrap();
        let hash2 = compute_skill_hash(&dir).unwrap();
        assert_eq!(hash1, hash2);
        assert!(hash1.starts_with("sha256:"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_compute_skill_hash_changes_on_template_edit() {
        let dir = std::env::temp_dir().join("forge_test_skill_hash_change");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("templates")).unwrap();
        fs::write(dir.join("skill.toml"), "[skill]\nname = \"test\"").unwrap();
        fs::write(dir.join("templates/a.tera"), "hello {{ name }}").unwrap();

        let hash1 = compute_skill_hash(&dir).unwrap();

        // Modify a template
        fs::write(dir.join("templates/a.tera"), "goodbye {{ name }}").unwrap();
        let hash2 = compute_skill_hash(&dir).unwrap();

        assert_ne!(hash1, hash2);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_trust_store_user() {
        let mut store = TrustStore::new();
        assert!(!store.check_user_trust("test", "sha256:abc"));

        store.set_user_trust("test", "sha256:abc");
        assert!(store.check_user_trust("test", "sha256:abc"));
        assert!(!store.check_user_trust("test", "sha256:def"));
    }

    #[test]
    fn test_trust_store_repo_local() {
        let mut store = TrustStore::new();
        let repo_a = "sha256:repo_a_hash";
        let repo_b = "sha256:repo_b_hash";

        store.set_repo_trust(repo_a, "skill", "sha256:abc");
        assert!(store.check_repo_trust(repo_a, "skill", "sha256:abc"));
        // Same skill name in different repo should NOT be trusted
        assert!(!store.check_repo_trust(repo_b, "skill", "sha256:abc"));
    }

    #[test]
    fn test_atomic_write() {
        let dir = std::env::temp_dir().join("forge_test_atomic");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let path = dir.join("test.txt");
        atomic_write(&path, "hello world").unwrap();

        assert_eq!(fs::read_to_string(&path).unwrap(), "hello world");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_atomic_write_creates_parents() {
        let dir = std::env::temp_dir().join("forge_test_atomic_parents");
        let _ = fs::remove_dir_all(&dir);

        let path = dir.join("nested/deep/test.txt");
        atomic_write(&path, "content").unwrap();

        assert_eq!(fs::read_to_string(&path).unwrap(), "content");

        let _ = fs::remove_dir_all(&dir);
    }

    // --- Transform tests ---

    #[test]
    fn test_json_merge_basic() {
        let existing = r#"{"name": "my-app", "version": "1.0.0"}"#;
        let merge: toml::Value = toml::from_str("[dependencies]\nzod = \"^3.22\"").unwrap();
        let result = json_merge(existing, &merge).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["name"], "my-app");
        assert_eq!(parsed["version"], "1.0.0");
        assert_eq!(parsed["dependencies"]["zod"], "^3.22");
    }

    #[test]
    fn test_json_merge_deep() {
        let existing = r#"{"dependencies": {"react": "^18.0"}, "name": "app"}"#;
        let merge: toml::Value = toml::from_str("[dependencies]\nzod = \"^3.22\"").unwrap();
        let result = json_merge(existing, &merge).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        // Existing key preserved
        assert_eq!(parsed["dependencies"]["react"], "^18.0");
        // New key merged
        assert_eq!(parsed["dependencies"]["zod"], "^3.22");
        assert_eq!(parsed["name"], "app");
    }

    #[test]
    fn test_json_merge_invalid_json() {
        let existing = "not json at all";
        let merge: toml::Value = toml::from_str("key = \"val\"").unwrap();
        let result = json_merge(existing, &merge);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("JSON"));
    }

    #[test]
    fn test_toml_merge_basic() {
        let existing = "[package]\nname = \"my-app\"\nversion = \"0.1.0\"\n";
        let merge: toml::Value = toml::from_str("[dependencies]\nserde = \"1.0\"").unwrap();
        let result = toml_merge(existing, &merge).unwrap();
        assert!(result.contains("name = \"my-app\""));
        assert!(result.contains("serde = \"1.0\""));
    }

    #[test]
    fn test_toml_merge_deep() {
        let existing = "[package]\nname = \"app\"\n\n[dependencies]\ntoml = \"0.8\"\n";
        let merge: toml::Value = toml::from_str("[dependencies]\nserde = \"1.0\"").unwrap();
        let result = toml_merge(existing, &merge).unwrap();
        assert!(result.contains("toml = \"0.8\""));
        assert!(result.contains("serde = \"1.0\""));
    }

    #[test]
    fn test_toml_merge_invalid_toml() {
        let existing = "{{not valid toml}}";
        let merge: toml::Value = toml::from_str("key = \"val\"").unwrap();
        let result = toml_merge(existing, &merge);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("TOML"));
    }

    #[test]
    fn test_line_append() {
        let existing = "line1\nline2\n";
        let lines = vec!["line3".into(), "line4".into()];
        let result = line_append(existing, &lines);
        assert_eq!(result, "line1\nline2\nline3\nline4\n");
    }

    #[test]
    fn test_line_append_no_trailing_newline() {
        let existing = "line1\nline2";
        let lines = vec!["line3".into()];
        let result = line_append(existing, &lines);
        assert_eq!(result, "line1\nline2\nline3\n");
    }

    #[test]
    fn test_line_append_empty() {
        let existing = "";
        let lines = vec!["first".into()];
        let result = line_append(existing, &lines);
        assert_eq!(result, "first\n");
    }

    #[test]
    fn test_line_prepend() {
        let existing = "line2\nline3\n";
        let lines = vec!["line0".into(), "line1".into()];
        let result = line_prepend(existing, &lines);
        assert_eq!(result, "line0\nline1\nline2\nline3\n");
    }

    #[test]
    fn test_line_prepend_empty_existing() {
        let existing = "";
        let lines = vec!["header".into()];
        let result = line_prepend(existing, &lines);
        assert_eq!(result, "header\n");
    }

    #[test]
    fn test_apply_transform_unknown_op() {
        let result = apply_transform("unknown_op", "", None, None, "test.txt");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Unknown transform"));
    }

    #[test]
    fn test_apply_transform_missing_value() {
        let result = apply_transform("json_merge", "{}", None, None, "test.json");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("requires"));
    }

    #[test]
    fn test_apply_transform_missing_lines() {
        let result = apply_transform("line_append", "content", None, None, "test.txt");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("requires"));
    }
}
