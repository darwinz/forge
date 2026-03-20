//! Bundle manifest editing — add packages to config/bundles/*.toml.
//!
//! Reads the TOML, modifies the relevant section, checks for duplicates,
//! and writes back with stable formatting.

use std::path::Path;

use crate::error::{ForgeError, ForgeResult};

/// Supported package sources for `add-package`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackageSource {
    BrewFormula,
    BrewCask,
    Npm,
    Go,
    Gem,
    UvTool,
    Manual,
}

impl PackageSource {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "brew" => Some(Self::BrewFormula),
            "brew-cask" | "cask" => Some(Self::BrewCask),
            "npm" => Some(Self::Npm),
            "go" => Some(Self::Go),
            "gem" => Some(Self::Gem),
            "uv-tool" | "uv" => Some(Self::UvTool),
            "manual" => Some(Self::Manual),
            _ => None,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::BrewFormula => "brew formula",
            Self::BrewCask => "brew cask",
            Self::Npm => "npm package",
            Self::Go => "go tool",
            Self::Gem => "gem package",
            Self::UvTool => "uv tool",
            Self::Manual => "manual entry",
        }
    }

    /// TOML section and key for this source.
    fn toml_location(&self) -> (&'static str, &'static str) {
        match self {
            Self::BrewFormula => ("brew", "formulae"),
            Self::BrewCask => ("brew", "casks"),
            Self::Npm => ("npm", "packages"),
            Self::Go => ("go", "tools"),
            Self::Gem => ("gem", "packages"),
            Self::UvTool => ("uv_tool", "tools"),
            Self::Manual => ("manual", ""),
        }
    }
}

/// Result of attempting to add a package.
#[derive(Debug)]
pub struct AddPackageResult {
    pub bundle_name: String,
    pub package: String,
    pub source: PackageSource,
    pub already_exists: bool,
    pub diff: String,
}

/// Check if a package already exists in a bundle TOML file.
pub fn check_duplicate(
    bundle_path: &Path,
    source: &PackageSource,
    package: &str,
) -> ForgeResult<bool> {
    let content = std::fs::read_to_string(bundle_path).map_err(|e| ForgeError::FileRead {
        path: bundle_path.to_path_buf(),
        source: e,
    })?;

    if *source == PackageSource::Manual {
        // Check manual entries by name
        return Ok(content.contains(&format!("name = \"{package}\"")));
    }

    let (section, key) = source.toml_location();
    let table: toml::Table = toml::from_str(&content).map_err(|e| ForgeError::TomlParse {
        path: bundle_path.to_path_buf(),
        source: e,
    })?;

    if let Some(sec) = table.get(section).and_then(|v| v.as_table()) {
        if let Some(arr) = sec.get(key).and_then(|v| v.as_array()) {
            return Ok(arr.iter().any(|v| v.as_str() == Some(package)));
        }
    }

    Ok(false)
}

/// Add a package to a bundle TOML file.
///
/// Returns the diff (before/after) for preview.
/// Does NOT write to disk — caller decides after preview.
pub fn plan_add_package(
    bundle_path: &Path,
    source: &PackageSource,
    package: &str,
    manual_instructions: Option<&str>,
    manual_check_command: Option<&str>,
) -> ForgeResult<AddPackageResult> {
    let original = std::fs::read_to_string(bundle_path).map_err(|e| ForgeError::FileRead {
        path: bundle_path.to_path_buf(),
        source: e,
    })?;

    // Extract bundle name
    let table: toml::Table = toml::from_str(&original).map_err(|e| ForgeError::TomlParse {
        path: bundle_path.to_path_buf(),
        source: e,
    })?;
    let bundle_name = table
        .get("bundle")
        .and_then(|v| v.as_table())
        .and_then(|t| t.get("name"))
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    // Check duplicate
    let already_exists = check_duplicate(bundle_path, source, package)?;
    if already_exists {
        return Ok(AddPackageResult {
            bundle_name,
            package: package.to_string(),
            source: source.clone(),
            already_exists: true,
            diff: String::new(),
        });
    }

    // Generate the new content
    let new_content = if *source == PackageSource::Manual {
        add_manual_entry(
            &original,
            package,
            manual_instructions.unwrap_or(&format!("Install {package} manually")),
            manual_check_command.unwrap_or(&format!("{package} --version")),
        )
    } else {
        add_array_entry(&original, source, package)?
    };

    // Generate diff
    let diff = generate_diff(&original, &new_content);

    Ok(AddPackageResult {
        bundle_name,
        package: package.to_string(),
        source: source.clone(),
        already_exists: false,
        diff,
    })
}

/// Write the updated content to the bundle file.
pub fn apply_add_package(
    bundle_path: &Path,
    source: &PackageSource,
    package: &str,
    manual_instructions: Option<&str>,
    manual_check_command: Option<&str>,
) -> ForgeResult<()> {
    let original = std::fs::read_to_string(bundle_path).map_err(|e| ForgeError::FileRead {
        path: bundle_path.to_path_buf(),
        source: e,
    })?;

    let new_content = if *source == PackageSource::Manual {
        add_manual_entry(
            &original,
            package,
            manual_instructions.unwrap_or(&format!("Install {package} manually")),
            manual_check_command.unwrap_or(&format!("{package} --version")),
        )
    } else {
        add_array_entry(&original, source, package)?
    };

    std::fs::write(bundle_path, new_content).map_err(|e| ForgeError::FileRead {
        path: bundle_path.to_path_buf(),
        source: e,
    })
}

/// Add a package to a TOML array (brew/npm/go/gem/uv_tool).
/// Does string-level editing to preserve formatting and comments.
fn add_array_entry(original: &str, source: &PackageSource, package: &str) -> ForgeResult<String> {
    let (section, key) = source.toml_location();

    // Find the array in the source text and append to it
    // Strategy: find `[section]` header, then find `key = [...]` and insert before the closing `]`

    let section_header = format!("[{section}]");
    let key_pattern = format!("{key} = ");

    // Check if section exists
    if !original.contains(&section_header) {
        // Append new section at the end
        let mut result = original.trim_end().to_string();
        result.push_str(&format!("\n\n[{section}]\n{key} = [\"{package}\"]\n"));
        return Ok(result);
    }

    // Find the array and insert the new entry
    let mut result = String::new();
    let mut found_section = false;
    let mut found_key = false;
    let mut inserted = false;

    for line in original.lines() {
        let trimmed = line.trim();

        if trimmed == section_header {
            found_section = true;
        } else if found_section && trimmed.starts_with('[') {
            // Moved to next section without finding the key — add it
            if !found_key && !inserted {
                result.push_str(&format!("{key} = [\"{package}\"]\n"));
                inserted = true;
            }
            found_section = false;
        }

        if found_section && trimmed.starts_with(&key_pattern) {
            found_key = true;

            // Handle single-line array: key = ["a", "b"]
            if trimmed.contains(']') {
                let close_pos = line.rfind(']').unwrap();
                let before_close = &line[..close_pos];
                let after_close = &line[close_pos..];
                // Check if array is empty
                if before_close.trim_end().ends_with('[') {
                    result.push_str(&format!("{}\"{}\"{}", before_close, package, after_close));
                } else {
                    result.push_str(&format!("{}, \"{}\"{}", before_close, package, after_close));
                }
                result.push('\n');
                inserted = true;
                continue;
            }
        }

        // Handle multi-line array: find the closing `]`
        if found_section && found_key && !inserted && trimmed == "]" {
            result.push_str(&format!("  \"{package}\",\n"));
            inserted = true;
        }

        result.push_str(line);
        result.push('\n');
    }

    // If we never found the key at all, append it to the section
    if !inserted {
        let mut new_result = result.trim_end().to_string();
        if found_section && !found_key {
            new_result.push_str(&format!("\n{key} = [\"{package}\"]\n"));
        } else {
            new_result.push_str(&format!("\n\n[{section}]\n{key} = [\"{package}\"]\n"));
        }
        return Ok(new_result);
    }

    Ok(result)
}

/// Add a manual entry to the TOML.
fn add_manual_entry(original: &str, name: &str, instructions: &str, check_command: &str) -> String {
    let mut result = original.trim_end().to_string();
    result.push_str(&format!(
        "\n\n[[manual]]\nname = \"{name}\"\ncheck_command = \"{check_command}\"\ninstall_instructions = \"{instructions}\"\n"
    ));
    result
}

/// Generate a simple line-level diff.
fn generate_diff(old: &str, new: &str) -> String {
    let old_lines: Vec<&str> = old.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();

    let mut diff = Vec::new();

    // Simple approach: show lines only in new that aren't in old
    for line in &new_lines {
        if !old_lines.contains(line) {
            diff.push(format!("+ {line}"));
        }
    }
    for line in &old_lines {
        if !new_lines.contains(line) {
            diff.push(format!("- {line}"));
        }
    }

    if diff.is_empty() {
        "  (no changes)".to_string()
    } else {
        diff.join("\n")
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_package_source_from_str() {
        assert_eq!(
            PackageSource::parse("brew"),
            Some(PackageSource::BrewFormula)
        );
        assert_eq!(PackageSource::parse("npm"), Some(PackageSource::Npm));
        assert_eq!(PackageSource::parse("go"), Some(PackageSource::Go));
        assert_eq!(PackageSource::parse("uv"), Some(PackageSource::UvTool));
        assert_eq!(PackageSource::parse("manual"), Some(PackageSource::Manual));
        assert_eq!(PackageSource::parse("unknown"), None);
    }

    #[test]
    fn test_check_duplicate_found() {
        let dir = std::env::temp_dir().join("forge_test_dup_check");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.toml");
        fs::write(
            &path,
            "[bundle]\nname = \"test\"\n\n[brew]\nformulae = [\"bat\", \"jq\"]\n",
        )
        .unwrap();

        assert!(check_duplicate(&path, &PackageSource::BrewFormula, "bat").unwrap());
        assert!(!check_duplicate(&path, &PackageSource::BrewFormula, "ripgrep").unwrap());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_check_duplicate_manual() {
        let dir = std::env::temp_dir().join("forge_test_dup_manual");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.toml");
        fs::write(&path, "[bundle]\nname = \"test\"\n\n[[manual]]\nname = \"claude\"\ncheck_command = \"claude --version\"\ninstall_instructions = \"install it\"\n").unwrap();

        assert!(check_duplicate(&path, &PackageSource::Manual, "claude").unwrap());
        assert!(!check_duplicate(&path, &PackageSource::Manual, "other").unwrap());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_add_array_entry_single_line() {
        let original = "[bundle]\nname = \"test\"\n\n[brew]\nformulae = [\"bat\", \"jq\"]\n";
        let result = add_array_entry(original, &PackageSource::BrewFormula, "ripgrep").unwrap();
        assert!(result.contains("ripgrep"));
        assert!(result.contains("bat"));
        assert!(result.contains("jq"));
    }

    #[test]
    fn test_add_array_entry_multi_line() {
        let original =
            "[bundle]\nname = \"test\"\n\n[brew]\nformulae = [\n  \"bat\",\n  \"jq\",\n]\n";
        let result = add_array_entry(original, &PackageSource::BrewFormula, "ripgrep").unwrap();
        assert!(result.contains("\"ripgrep\""));
        assert!(result.contains("bat"));
    }

    #[test]
    fn test_add_array_entry_new_section() {
        let original = "[bundle]\nname = \"test\"\n";
        let result = add_array_entry(original, &PackageSource::Npm, "typescript").unwrap();
        assert!(result.contains("[npm]"));
        assert!(result.contains("\"typescript\""));
    }

    #[test]
    fn test_add_manual_entry() {
        let original = "[bundle]\nname = \"test\"\n";
        let result = add_manual_entry(original, "mytool", "install it", "mytool --version");
        assert!(result.contains("[[manual]]"));
        assert!(result.contains("name = \"mytool\""));
        assert!(result.contains("install_instructions"));
    }

    #[test]
    fn test_generate_diff() {
        let old = "line1\nline2\n";
        let new = "line1\nline2\nline3\n";
        let diff = generate_diff(old, new);
        assert!(diff.contains("+ line3"));
    }

    #[test]
    fn test_plan_add_package_detects_duplicate() {
        let dir = std::env::temp_dir().join("forge_test_plan_dup");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.toml");
        fs::write(
            &path,
            "[bundle]\nname = \"test\"\n\n[brew]\nformulae = [\"bat\"]\n",
        )
        .unwrap();

        let result =
            plan_add_package(&path, &PackageSource::BrewFormula, "bat", None, None).unwrap();
        assert!(result.already_exists);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_plan_add_package_new() {
        let dir = std::env::temp_dir().join("forge_test_plan_new");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.toml");
        fs::write(
            &path,
            "[bundle]\nname = \"core\"\n\n[brew]\nformulae = [\"bat\"]\n",
        )
        .unwrap();

        let result =
            plan_add_package(&path, &PackageSource::BrewFormula, "ripgrep", None, None).unwrap();
        assert!(!result.already_exists);
        assert!(result.diff.contains("ripgrep"));
        assert_eq!(result.bundle_name, "core");

        let _ = fs::remove_dir_all(&dir);
    }
}
