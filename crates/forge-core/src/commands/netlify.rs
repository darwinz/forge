//! Netlify CLI integration — doctor, env-diff.
//!
//! Uses the `netlify` CLI as the backend. All calls go through CommandRunner.
//! Auth checks use `netlify status` or file existence — forge never reads tokens.

use std::collections::BTreeMap;
use std::path::Path;

use crate::error::ForgeResult;
use crate::exec::CommandRunner;

// ---------------------------------------------------------------------------
// Doctor
// ---------------------------------------------------------------------------

/// Result of a Netlify doctor check.
#[derive(Debug)]
pub struct NetlifyDoctorResult {
    pub cli_installed: bool,
    pub cli_version: Option<String>,
    pub authenticated: bool,
    pub auth_user: Option<String>,
    pub site_linked: bool,
    pub site_name: Option<String>,
    pub issues: Vec<String>,
    pub suggestions: Vec<String>,
}

/// Run Netlify doctor checks.
pub fn doctor(runner: &dyn CommandRunner) -> NetlifyDoctorResult {
    let mut result = NetlifyDoctorResult {
        cli_installed: false,
        cli_version: None,
        authenticated: false,
        auth_user: None,
        site_linked: false,
        site_name: None,
        issues: Vec::new(),
        suggestions: Vec::new(),
    };

    // 1. CLI installed?
    match runner.run("netlify", &["--version"]) {
        Ok(output) if output.success() => {
            result.cli_installed = true;
            result.cli_version = output
                .stdout
                .lines()
                .next()
                .map(|l| l.trim().to_string())
                .filter(|v| !v.is_empty());
        }
        _ => {
            result.issues.push("Netlify CLI is not installed.".into());
            result
                .suggestions
                .push("Install with: npm i -g netlify-cli".into());
            return result;
        }
    }

    if runner.is_dry_run() {
        result
            .issues
            .push("[dry-run] Skipped auth and site checks.".into());
        return result;
    }

    // 2. Authenticated? Parse `netlify status` output
    match runner.run("netlify", &["status"]) {
        Ok(output) if output.success() => {
            let stdout = &output.stdout;
            if stdout.contains("Not logged in") || stdout.contains("Login with") {
                result.issues.push("Not logged in to Netlify.".into());
                result.suggestions.push("Run: netlify login".into());
            } else {
                result.authenticated = true;
                // Try to extract email/name from status output
                result.auth_user = extract_status_field(stdout, "Email");

                // 3. Site linked?
                if stdout.contains("Current site:") || stdout.contains("Site:") {
                    result.site_linked = true;
                    result.site_name = extract_status_field(stdout, "Site")
                        .or_else(|| extract_status_field(stdout, "Current site"));
                }
            }
        }
        _ => {
            result.issues.push("Could not check Netlify status.".into());
            result.suggestions.push("Run: netlify login".into());
        }
    }

    // Fallback site link check via .netlify/state.json
    if !result.site_linked && Path::new(".netlify/state.json").exists() {
        result.site_linked = true;
        result.site_name = Some("linked (via .netlify/state.json)".into());
    }

    if !result.site_linked {
        result.issues.push("No Netlify site linked.".into());
        result.suggestions.push("Run: netlify link".into());
    }

    result
}

/// Extract a field value from `netlify status` output.
/// Lines look like: "  Email:   user@example.com"
fn extract_status_field(stdout: &str, field: &str) -> Option<String> {
    for line in stdout.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix(field) {
            let rest = rest.trim_start_matches(':').trim();
            if !rest.is_empty() {
                return Some(rest.to_string());
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Env diff
// ---------------------------------------------------------------------------

/// Result of comparing local vs Netlify env vars.
#[derive(Debug)]
pub struct EnvDiffResult {
    pub local_only: Vec<String>,
    pub remote_only: Vec<String>,
    pub both: Vec<String>,
    pub local_source: String,
    pub remote_available: bool,
}

/// Compare local .env files against Netlify environment variables.
pub fn env_diff(runner: &dyn CommandRunner) -> ForgeResult<EnvDiffResult> {
    let local_keys = read_local_env_keys();
    let local_source = detect_env_source();
    let (remote_keys, remote_available) = read_remote_env_keys(runner);

    let local_set: std::collections::BTreeSet<&str> =
        local_keys.iter().map(|s| s.as_str()).collect();
    let remote_set: std::collections::BTreeSet<&str> =
        remote_keys.iter().map(|s| s.as_str()).collect();

    let local_only: Vec<String> = local_set
        .difference(&remote_set)
        .map(|s| s.to_string())
        .collect();
    let remote_only: Vec<String> = remote_set
        .difference(&local_set)
        .map(|s| s.to_string())
        .collect();
    let both: Vec<String> = local_set
        .intersection(&remote_set)
        .map(|s| s.to_string())
        .collect();

    Ok(EnvDiffResult {
        local_only,
        remote_only,
        both,
        local_source,
        remote_available,
    })
}

fn read_local_env_keys() -> Vec<String> {
    let files = [".env", ".env.local", ".env.development"];
    let mut keys = BTreeMap::new();

    for file in &files {
        if let Ok(content) = std::fs::read_to_string(file) {
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    continue;
                }
                if let Some(eq_pos) = trimmed.find('=') {
                    let key = trimmed[..eq_pos].trim().to_string();
                    if !key.is_empty() {
                        keys.entry(key).or_insert(());
                    }
                }
            }
        }
    }

    keys.into_keys().collect()
}

fn detect_env_source() -> String {
    let files = [".env", ".env.local", ".env.development"];
    let found: Vec<&str> = files
        .iter()
        .filter(|f| Path::new(f).exists())
        .copied()
        .collect();
    if found.is_empty() {
        "no .env files found".to_string()
    } else {
        found.join(", ")
    }
}

fn read_remote_env_keys(runner: &dyn CommandRunner) -> (Vec<String>, bool) {
    if runner.is_dry_run() {
        return (Vec::new(), false);
    }

    match runner.run("netlify", &["env:list"]) {
        Ok(output) if output.success() => {
            let keys = parse_netlify_env_list(&output.stdout);
            (keys, true)
        }
        _ => (Vec::new(), false),
    }
}

/// Parse `netlify env:list` output to extract variable names.
///
/// Output lines look like:
///
/// ```text
/// .-------------------------------.
/// | Key          | Value          |
/// |--------------|----------------|
/// | DATABASE_URL | (set)          |
/// '-------------------------------'
/// ```
///
/// Or in newer versions, simpler list format.
pub fn parse_netlify_env_list(stdout: &str) -> Vec<String> {
    stdout
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim().trim_start_matches('|').trim();
            // Skip header, borders, empty
            if trimmed.is_empty()
                || trimmed.starts_with('-')
                || trimmed.starts_with('.')
                || trimmed.starts_with('\'')
                || trimmed.starts_with("Key")
            {
                return None;
            }
            // Extract key from "KEY | value" or "KEY  (set)"
            let token = trimmed.split_whitespace().next()?;
            // Env var names: start with letter, contain underscore or all uppercase
            let starts_with_letter = token
                .chars()
                .next()
                .map(|c| c.is_ascii_alphabetic())
                .unwrap_or(false);
            let looks_like_env = token.contains('_')
                || token
                    .chars()
                    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_');

            if token.len() > 1 && starts_with_letter && looks_like_env {
                Some(token.to_string())
            } else {
                None
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_doctor_dry_run() {
        use crate::exec::DryRunRunner;
        let runner = DryRunRunner;
        let result = doctor(&runner);
        assert!(result.cli_installed);
        assert!(result.issues.iter().any(|i| i.contains("[dry-run]")));
    }

    #[test]
    fn test_extract_status_field() {
        let stdout = "\
  Email:   user@example.com
  Teams:   My Team
  Site:    my-cool-site
";
        assert_eq!(
            extract_status_field(stdout, "Email"),
            Some("user@example.com".to_string())
        );
        assert_eq!(
            extract_status_field(stdout, "Site"),
            Some("my-cool-site".to_string())
        );
        assert_eq!(extract_status_field(stdout, "Missing"), None);
    }

    #[test]
    fn test_parse_netlify_env_list_table_format() {
        let output = "\
.-------------------------------.
| Key            | Value        |
|----------------|--------------|
| DATABASE_URL   | (set)        |
| API_KEY        | (set)        |
'-------------------------------'
";
        let keys = parse_netlify_env_list(output);
        assert!(keys.contains(&"DATABASE_URL".to_string()));
        assert!(keys.contains(&"API_KEY".to_string()));
    }

    #[test]
    fn test_parse_netlify_env_list_empty() {
        assert!(parse_netlify_env_list("").is_empty());
        assert!(parse_netlify_env_list("No environment variables").is_empty());
    }

    #[test]
    fn test_env_diff_dry_run() {
        use crate::exec::DryRunRunner;
        let runner = DryRunRunner;
        let result = env_diff(&runner).unwrap();
        assert!(!result.remote_available);
    }
}
