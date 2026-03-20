use super::metadata::DiscoveredSkill;

/// Validation result for a skill
#[derive(Debug)]
pub struct ValidationResult {
    pub skill_name: String,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl ValidationResult {
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }
}

/// Validate a skill manifest
pub fn validate_skill(skill: &DiscoveredSkill) -> ValidationResult {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    let info = &skill.manifest.skill;

    // Required fields
    if info.name.is_empty() {
        errors.push("skill name is required".to_string());
    }
    if info.version.is_empty() {
        errors.push("skill version is required".to_string());
    }
    if info.description.is_empty() {
        errors.push("skill description is required".to_string());
    }

    // Version format (semver-ish)
    if !info.version.is_empty() {
        let parts: Vec<&str> = info.version.split('.').collect();
        if parts.len() != 3 || !parts.iter().all(|p| p.parse::<u32>().is_ok()) {
            warnings.push(format!("version '{}' is not semver", info.version));
        }
    }

    // Execution section
    if let Some(ref exec) = info.execution {
        // Check entrypoint exists
        let entrypoint = skill.path.join(&exec.entrypoint);
        if !entrypoint.exists() {
            errors.push(format!("entrypoint '{}' not found at {:?}", exec.entrypoint, entrypoint));
        }

        // Validate exec type
        match exec.exec_type.as_str() {
            "script" | "binary" | "wasm" => {}
            other => errors.push(format!("unknown execution type: {other}")),
        }
    }

    // Permissions
    if let Some(ref perms) = info.permissions {
        if perms.execute_commands && !perms.read_files {
            warnings.push("execute_commands=true without read_files=true is unusual".to_string());
        }
    } else {
        warnings.push("no permissions section defined".to_string());
    }

    // Metadata
    if info.metadata.is_none() {
        warnings.push("no metadata section (tags, category)".to_string());
    }

    ValidationResult {
        skill_name: info.name.clone(),
        errors,
        warnings,
    }
}
