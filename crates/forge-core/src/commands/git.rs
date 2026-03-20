use std::collections::BTreeMap;
use std::path::Path;

use crate::error::{ForgeError, ForgeResult};
use crate::exec::CommandRunner;

/// A parsed git alias from the config file
#[derive(Debug, Clone)]
pub struct GitAlias {
    pub name: String,
    pub value: String,
}

/// Parse git aliases from a TOML file
///
/// Expected format:
/// ```toml
/// [aliases]
/// co = "checkout"
/// s = "status -s"
/// ```
pub fn load_aliases(path: &Path) -> ForgeResult<Vec<GitAlias>> {
    let content = std::fs::read_to_string(path).map_err(|e| ForgeError::FileRead {
        path: path.to_path_buf(),
        source: e,
    })?;

    let table: toml::Table = toml::from_str(&content).map_err(|e| ForgeError::TomlParse {
        path: path.to_path_buf(),
        source: e,
    })?;

    let aliases_table = table
        .get("aliases")
        .and_then(|v| v.as_table())
        .ok_or_else(|| {
            ForgeError::Config(format!("missing [aliases] table in {}", path.display()))
        })?;

    let mut aliases = Vec::new();
    for (name, value) in aliases_table {
        if let Some(val) = value.as_str() {
            aliases.push(GitAlias {
                name: name.clone(),
                value: val.to_string(),
            });
        }
    }

    // Sort by name for stable output
    aliases.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(aliases)
}

/// Format aliases for display
pub fn format_aliases(aliases: &[GitAlias]) -> String {
    if aliases.is_empty() {
        return "No aliases configured.".to_string();
    }

    let max_name_len = aliases.iter().map(|a| a.name.len()).max().unwrap_or(0);
    let mut lines = Vec::new();
    for alias in aliases {
        lines.push(format!(
            "  git {:<width$} → {}",
            alias.name,
            alias.value,
            width = max_name_len
        ));
    }
    lines.join("\n")
}

/// Result of diffing configured aliases against current git config.
pub struct AliasDiff {
    pub to_add: Vec<GitAlias>,
    pub to_change: Vec<(GitAlias, String)>,
    pub unchanged: Vec<GitAlias>,
}

/// Compute what changes setup-aliases would make.
pub fn diff_aliases(runner: &dyn CommandRunner, aliases: &[GitAlias]) -> ForgeResult<AliasDiff> {
    // Read current git aliases
    let current = read_current_aliases(runner)?;

    let mut to_add = Vec::new();
    let mut to_change = Vec::new();
    let mut unchanged = Vec::new();

    for alias in aliases {
        match current.get(&alias.name) {
            None => to_add.push(alias.clone()),
            Some(existing) if existing != &alias.value => {
                to_change.push((alias.clone(), existing.clone()));
            }
            Some(_) => unchanged.push(alias.clone()),
        }
    }

    Ok(AliasDiff {
        to_add,
        to_change,
        unchanged,
    })
}

/// Read currently configured git aliases from git config --global
fn read_current_aliases(runner: &dyn CommandRunner) -> ForgeResult<BTreeMap<String, String>> {
    let output = runner.run("git", &["config", "--global", "--get-regexp", "^alias\\."])?;

    let mut map = BTreeMap::new();

    // If dry-run, we get empty output — return empty map
    if runner.is_dry_run() || output.stdout.is_empty() {
        return Ok(map);
    }

    // Output format: alias.name value
    for line in output.stdout.lines() {
        if let Some(rest) = line.strip_prefix("alias.") {
            // Split on first space
            if let Some(space_idx) = rest.find(' ') {
                let name = &rest[..space_idx];
                let value = &rest[space_idx + 1..];
                map.insert(name.to_string(), value.to_string());
            }
        }
    }

    Ok(map)
}

/// Apply aliases to git config --global
pub fn apply_aliases(
    runner: &dyn CommandRunner,
    aliases: &[GitAlias],
) -> ForgeResult<Vec<ApplyResult>> {
    let mut results = Vec::new();

    for alias in aliases {
        let key = format!("alias.{}", alias.name);
        let output = runner.run("git", &["config", "--global", &key, &alias.value])?;

        results.push(ApplyResult {
            name: alias.name.clone(),
            success: runner.is_dry_run() || output.success(),
        });
    }

    Ok(results)
}

/// Result of applying a single alias
#[derive(Debug)]
pub struct ApplyResult {
    pub name: String,
    pub success: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_load_aliases_valid() {
        let dir = std::env::temp_dir().join("forge_test_git_aliases");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("aliases.toml");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(
            f,
            r#"[aliases]
co = "checkout"
s = "status -s"
lg = "log --oneline"
"#
        )
        .unwrap();

        let aliases = load_aliases(&path).unwrap();
        assert_eq!(aliases.len(), 3);
        // Sorted by name
        assert_eq!(aliases[0].name, "co");
        assert_eq!(aliases[0].value, "checkout");
        assert_eq!(aliases[1].name, "lg");
        assert_eq!(aliases[2].name, "s");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_load_aliases_missing_table() {
        let dir = std::env::temp_dir().join("forge_test_git_no_table");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("empty.toml");
        std::fs::write(&path, "[other]\nfoo = \"bar\"\n").unwrap();

        let result = load_aliases(&path);
        assert!(result.is_err());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_load_aliases_empty_table() {
        let dir = std::env::temp_dir().join("forge_test_git_empty");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("empty_aliases.toml");
        std::fs::write(&path, "[aliases]\n").unwrap();

        let aliases = load_aliases(&path).unwrap();
        assert!(aliases.is_empty());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_format_aliases() {
        let aliases = vec![
            GitAlias {
                name: "co".to_string(),
                value: "checkout".to_string(),
            },
            GitAlias {
                name: "s".to_string(),
                value: "status -s".to_string(),
            },
        ];

        let output = format_aliases(&aliases);
        assert!(output.contains("git co"));
        assert!(output.contains("checkout"));
        assert!(output.contains("git s"));
        assert!(output.contains("status -s"));
    }

    #[test]
    fn test_format_aliases_empty() {
        let output = format_aliases(&[]);
        assert_eq!(output, "No aliases configured.");
    }

    #[test]
    fn test_diff_aliases_all_new() {
        use crate::exec::DryRunRunner;

        let runner = DryRunRunner;
        let aliases = vec![GitAlias {
            name: "co".to_string(),
            value: "checkout".to_string(),
        }];

        // DryRunRunner returns empty git config, so everything is "to add"
        let diff = diff_aliases(&runner, &aliases).unwrap();
        assert_eq!(diff.to_add.len(), 1);
        assert!(diff.to_change.is_empty());
        assert!(diff.unchanged.is_empty());
    }
}
