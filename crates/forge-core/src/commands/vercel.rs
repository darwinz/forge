//! Vercel CLI integration — doctor, env-diff, deploy.
//!
//! Uses the `vercel` CLI as the backend. All calls go through CommandRunner.
//! Auth checks use `vercel whoami` — forge never reads token files directly.

use std::collections::BTreeMap;
use std::path::Path;

use crate::error::{ForgeError, ForgeResult};
use crate::exec::CommandRunner;

// ---------------------------------------------------------------------------
// Doctor
// ---------------------------------------------------------------------------

/// Result of a Vercel doctor check.
#[derive(Debug)]
pub struct VercelDoctorResult {
    pub cli_installed: bool,
    pub cli_version: Option<String>,
    pub authenticated: bool,
    pub auth_user: Option<String>,
    pub project_linked: bool,
    pub project_name: Option<String>,
    pub framework: Option<String>,
    pub issues: Vec<String>,
    pub suggestions: Vec<String>,
}

/// Run comprehensive Vercel doctor checks.
pub fn doctor(runner: &dyn CommandRunner) -> VercelDoctorResult {
    let mut result = VercelDoctorResult {
        cli_installed: false,
        cli_version: None,
        authenticated: false,
        auth_user: None,
        project_linked: false,
        project_name: None,
        framework: None,
        issues: Vec::new(),
        suggestions: Vec::new(),
    };

    // 1. CLI installed?
    match runner.run("vercel", &["--version"]) {
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
            result.issues.push("Vercel CLI is not installed.".into());
            result
                .suggestions
                .push("Install with: npm i -g vercel".into());
            return result;
        }
    }

    if runner.is_dry_run() {
        result
            .issues
            .push("[dry-run] Skipped auth and project checks.".into());
        return result;
    }

    // 2. Authenticated?
    match runner.run("vercel", &["whoami"]) {
        Ok(output) if output.success() => {
            result.authenticated = true;
            // whoami output: "Vercel CLI x.y.z\nusername"
            result.auth_user = output
                .stdout
                .lines()
                .find(|l| !l.starts_with("Vercel CLI"))
                .map(|l| l.trim().to_string())
                .filter(|u| !u.is_empty());
        }
        _ => {
            result.issues.push("Not logged in to Vercel.".into());
            result.suggestions.push("Run: vercel login".into());
        }
    }

    // 3. Project linked?
    let project_json = Path::new(".vercel/project.json");
    if project_json.exists() {
        result.project_linked = true;
        // Try to read project name from the JSON (simple parse)
        if let Ok(content) = std::fs::read_to_string(project_json) {
            result.project_name = extract_json_field(&content, "projectId");
        }
    } else {
        result.issues.push("Not linked to a Vercel project.".into());
        result.suggestions.push("Run: vercel link".into());
    }

    // 4. Framework detection
    if Path::new("next.config.ts").exists()
        || Path::new("next.config.js").exists()
        || Path::new("next.config.mjs").exists()
    {
        result.framework = Some("Next.js".into());
    } else if Path::new("vite.config.ts").exists() || Path::new("vite.config.js").exists() {
        result.framework = Some("Vite".into());
    } else if Path::new("nuxt.config.ts").exists() {
        result.framework = Some("Nuxt".into());
    } else if Path::new("svelte.config.js").exists() {
        result.framework = Some("SvelteKit".into());
    } else if Path::new("astro.config.mjs").exists() || Path::new("astro.config.ts").exists() {
        result.framework = Some("Astro".into());
    } else if Path::new("remix.config.js").exists() {
        result.framework = Some("Remix".into());
    }

    // 5. Check for common misconfigurations
    if !Path::new(".gitignore").exists() {
        result
            .suggestions
            .push("No .gitignore found — consider adding one.".into());
    } else if let Ok(content) = std::fs::read_to_string(".gitignore") {
        if !content.contains(".env") {
            result
                .suggestions
                .push(".gitignore does not mention .env files — secrets may be committed.".into());
        }
        if !content.contains(".vercel") {
            result
                .suggestions
                .push(".gitignore does not mention .vercel/ — link state may be committed.".into());
        }
    }

    result
}

/// Simple JSON string field extractor (avoids serde_json dependency).
fn extract_json_field(json: &str, field: &str) -> Option<String> {
    let pattern = format!("\"{}\"", field);
    let start = json.find(&pattern)?;
    let after_key = &json[start + pattern.len()..];
    // Skip ": or " :
    let colon = after_key.find(':')?;
    let after_colon = after_key[colon + 1..].trim_start();
    if after_colon.starts_with('"') {
        let value_start = 1;
        let value_end = after_colon[value_start..].find('"')?;
        Some(after_colon[value_start..value_start + value_end].to_string())
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Env diff
// ---------------------------------------------------------------------------

/// An environment variable entry for diffing.
#[derive(Debug, Clone)]
pub struct EnvEntry {
    pub key: String,
    pub value: Option<String>, // None = present but value unknown (remote vars)
}

/// Result of comparing local vs remote env vars.
#[derive(Debug)]
pub struct EnvDiffResult {
    pub local_only: Vec<String>,
    pub remote_only: Vec<String>,
    pub both: Vec<String>,
    pub local_source: String,
    pub remote_available: bool,
}

/// Compare local .env files against Vercel remote environment variables.
pub fn env_diff(runner: &dyn CommandRunner) -> ForgeResult<EnvDiffResult> {
    // 1. Read local env vars from .env files
    let local_keys = read_local_env_keys();
    let local_source = detect_env_source();

    // 2. Read remote env var names from Vercel CLI
    let (remote_keys, remote_available) = read_remote_env_keys(runner);

    // 3. Diff
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

/// Read env var keys from local .env files.
/// Checks .env.local, .env, .env.development in priority order.
fn read_local_env_keys() -> Vec<String> {
    let files = [
        ".env.local",
        ".env",
        ".env.development",
        ".env.development.local",
    ];
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

/// Detect which .env file(s) are present for display.
fn detect_env_source() -> String {
    let files = [
        ".env.local",
        ".env",
        ".env.development",
        ".env.development.local",
    ];
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

/// Read env var names from Vercel remote via `vercel env ls`.
/// Returns (keys, available) where available=false if the command fails.
fn read_remote_env_keys(runner: &dyn CommandRunner) -> (Vec<String>, bool) {
    if runner.is_dry_run() {
        return (Vec::new(), false);
    }

    match runner.run("vercel", &["env", "ls"]) {
        Ok(output) if output.success() => {
            let keys = parse_vercel_env_ls(&output.stdout);
            (keys, true)
        }
        _ => (Vec::new(), false),
    }
}

/// Parse `vercel env ls` output to extract variable names.
///
/// Output format varies by version but typically includes lines like:
///
/// ```text
/// > Name                     Environment  Value
/// DATABASE_URL               Production   Encrypted
/// NEXT_PUBLIC_API_URL        Preview      ***
/// ```
///
/// We extract the first whitespace-delimited token from lines that look
/// like env var entries (ALL_CAPS or contain underscore, not a header/divider).
pub fn parse_vercel_env_ls(stdout: &str) -> Vec<String> {
    stdout
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter(|l| !l.contains("Environment") || !l.contains("Value")) // skip header
        .filter(|l| !l.starts_with('>') && !l.starts_with("Vercel CLI"))
        .filter_map(|line| {
            let token = line.split_whitespace().next()?;
            // Env var names: start with a letter, contain underscore or are ALL_CAPS
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
// Deploy
// ---------------------------------------------------------------------------

/// Build the vercel deploy command.
/// Returns the args to pass to `vercel`.
pub fn deploy_args(prod: bool) -> Vec<&'static str> {
    if prod {
        vec!["--prod"]
    } else {
        vec!["deploy"]
    }
}

/// Execute a Vercel deploy.
pub fn deploy(runner: &dyn CommandRunner, prod: bool) -> ForgeResult<String> {
    let args = deploy_args(prod);
    let output = runner.run("vercel", &args)?;

    if runner.is_dry_run() {
        return Ok(String::new());
    }

    if output.success() {
        // vercel deploy outputs the URL on success
        let url = output
            .stdout
            .lines()
            .filter(|l| l.starts_with("http"))
            .last()
            .unwrap_or("deployed successfully");
        Ok(url.to_string())
    } else {
        let msg = output.stderr.lines().next().unwrap_or("deploy failed");
        Err(ForgeError::CommandFailed(msg.to_string()))
    }
}

// ---------------------------------------------------------------------------
// Status (latest deployment)
// ---------------------------------------------------------------------------

/// Result of checking deployment status.
#[derive(Debug)]
pub struct VercelStatusResult {
    pub available: bool,
    pub output: String,
}

/// Get latest deployment status via `vercel ls`.
pub fn status(runner: &dyn CommandRunner) -> ForgeResult<VercelStatusResult> {
    let output = runner.run("vercel", &["ls", "--limit", "5"])?;

    if runner.is_dry_run() {
        return Ok(VercelStatusResult {
            available: false,
            output: String::new(),
        });
    }

    if output.success() {
        Ok(VercelStatusResult {
            available: true,
            output: output.stdout,
        })
    } else {
        let msg = output
            .stderr
            .lines()
            .next()
            .unwrap_or("could not list deployments");
        Ok(VercelStatusResult {
            available: false,
            output: msg.to_string(),
        })
    }
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
        // DryRunRunner makes vercel --version "succeed"
        assert!(result.cli_installed);
        assert!(result.issues.iter().any(|i| i.contains("[dry-run]")));
    }

    #[test]
    fn test_extract_json_field() {
        let json = r#"{"projectId": "prj_abc123", "orgId": "team_xyz"}"#;
        assert_eq!(
            extract_json_field(json, "projectId"),
            Some("prj_abc123".to_string())
        );
        assert_eq!(
            extract_json_field(json, "orgId"),
            Some("team_xyz".to_string())
        );
        assert_eq!(extract_json_field(json, "missing"), None);
    }

    #[test]
    fn test_extract_json_field_with_spaces() {
        let json = r#"{ "projectId" : "prj_abc123" }"#;
        assert_eq!(
            extract_json_field(json, "projectId"),
            Some("prj_abc123".to_string())
        );
    }

    #[test]
    fn test_parse_vercel_env_ls_typical() {
        let output = "\
Vercel CLI 39.3.0
> Environment Variables for project my-app
  Name                     Environment  Value
  DATABASE_URL             Production   Encrypted
  NEXT_PUBLIC_API_URL      Preview      ***
  API_SECRET               Development  Encrypted
";
        let keys = parse_vercel_env_ls(output);
        assert!(keys.contains(&"DATABASE_URL".to_string()));
        assert!(keys.contains(&"NEXT_PUBLIC_API_URL".to_string()));
        assert!(keys.contains(&"API_SECRET".to_string()));
    }

    #[test]
    fn test_parse_vercel_env_ls_empty() {
        let output = "Vercel CLI 39.3.0\n";
        let keys = parse_vercel_env_ls(output);
        assert!(keys.is_empty());
    }

    #[test]
    fn test_parse_vercel_env_ls_filters_noise() {
        let output = "\
Vercel CLI 39.3.0
> Linked to my-project
DATABASE_URL  Production  Encrypted
some random line here
A  one-char-name
";
        let keys = parse_vercel_env_ls(output);
        assert!(keys.contains(&"DATABASE_URL".to_string()));
        // "some", "random", etc. should not appear
        assert!(!keys.iter().any(|k| k == "some"));
        assert!(!keys.iter().any(|k| k == "random"));
    }

    #[test]
    fn test_deploy_args_preview() {
        let args = deploy_args(false);
        assert_eq!(args, vec!["deploy"]);
    }

    #[test]
    fn test_deploy_args_prod() {
        let args = deploy_args(true);
        assert_eq!(args, vec!["--prod"]);
    }

    #[test]
    fn test_deploy_dry_run() {
        use crate::exec::DryRunRunner;
        let runner = DryRunRunner;
        let result = deploy(&runner, false).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_env_diff_dry_run() {
        use crate::exec::DryRunRunner;
        let runner = DryRunRunner;
        let result = env_diff(&runner).unwrap();
        assert!(!result.remote_available);
    }

    #[test]
    fn test_status_dry_run() {
        use crate::exec::DryRunRunner;
        let runner = DryRunRunner;
        let result = status(&runner).unwrap();
        assert!(!result.available);
    }
}
