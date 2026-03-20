//! Supabase CLI integration — doctor, migration status, local reset.
//!
//! Uses the `supabase` CLI as the backend. All calls go through CommandRunner.
//! Auth checks use file existence — forge never reads token contents.

use std::path::Path;

use crate::error::{ForgeError, ForgeResult};
use crate::exec::CommandRunner;

// ---------------------------------------------------------------------------
// Doctor
// ---------------------------------------------------------------------------

/// Result of a Supabase doctor check.
#[derive(Debug)]
pub struct SupabaseDoctorResult {
    pub cli_installed: bool,
    pub cli_version: Option<String>,
    pub docker_running: bool,
    pub project_initialized: bool,
    pub project_linked: bool,
    pub local_running: bool,
    pub local_status: Option<String>,
    pub migration_count: usize,
    pub issues: Vec<String>,
    pub suggestions: Vec<String>,
}

/// Run comprehensive Supabase doctor checks.
pub fn doctor(runner: &dyn CommandRunner) -> SupabaseDoctorResult {
    let mut result = SupabaseDoctorResult {
        cli_installed: false,
        cli_version: None,
        docker_running: false,
        project_initialized: false,
        project_linked: false,
        local_running: false,
        local_status: None,
        migration_count: 0,
        issues: Vec::new(),
        suggestions: Vec::new(),
    };

    // 1. CLI installed?
    match runner.run("supabase", &["--version"]) {
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
            result.issues.push("Supabase CLI is not installed.".into());
            result
                .suggestions
                .push("Install with: brew install supabase".into());
            return result;
        }
    }

    if runner.is_dry_run() {
        result
            .issues
            .push("[dry-run] Skipped Docker, project, and status checks.".into());
        return result;
    }

    // 2. Docker running?
    match runner.run("docker", &["info"]) {
        Ok(output) if output.success() => {
            result.docker_running = true;
        }
        _ => {
            result.issues.push("Docker is not running.".into());
            result
                .suggestions
                .push("Start Docker Desktop or OrbStack before using Supabase locally.".into());
        }
    }

    // 3. Project initialized? (supabase/config.toml exists)
    if Path::new("supabase/config.toml").exists() {
        result.project_initialized = true;
    } else {
        result
            .issues
            .push("No Supabase project found (supabase/config.toml missing).".into());
        result.suggestions.push("Run: supabase init".into());
    }

    // 4. Project linked? (supabase/.temp/project-ref exists)
    if Path::new("supabase/.temp/project-ref").exists() {
        result.project_linked = true;
    } else if result.project_initialized {
        result
            .issues
            .push("Project not linked to a remote Supabase project.".into());
        result
            .suggestions
            .push("Run: supabase link --project-ref <your-project-ref>".into());
    }

    // 5. Local stack running? (supabase status)
    if result.docker_running && result.project_initialized {
        match runner.run("supabase", &["status"]) {
            Ok(output) if output.success() => {
                result.local_running = true;
                result.local_status = Some(summarize_status(&output.stdout));
            }
            Ok(output) => {
                // Status command ran but reported not running
                let stderr = output.stderr.trim().to_string();
                if stderr.contains("not running") || stderr.contains("Cannot connect") {
                    result.local_running = false;
                } else {
                    result.local_status = Some(stderr);
                }
            }
            _ => {}
        }
    }

    // 6. Count local migrations
    if result.project_initialized {
        result.migration_count = count_local_migrations();
    }

    result
}

/// Extract a one-line summary from `supabase status` output.
fn summarize_status(stdout: &str) -> String {
    let mut services = Vec::new();
    for line in stdout.lines() {
        let trimmed = line.trim();
        // Status output has lines like "         API URL: http://..."
        if trimmed.starts_with("API URL:") {
            services.push("API");
        } else if trimmed.starts_with("DB URL:") {
            services.push("DB");
        } else if trimmed.starts_with("Studio URL:") {
            services.push("Studio");
        } else if trimmed.starts_with("Inbucket URL:") {
            services.push("Inbucket");
        }
    }
    if services.is_empty() {
        "running".to_string()
    } else {
        format!("running ({})", services.join(", "))
    }
}

// ---------------------------------------------------------------------------
// Migration status
// ---------------------------------------------------------------------------

/// A local migration file.
#[derive(Debug, Clone)]
pub struct LocalMigration {
    pub timestamp: String,
    pub name: String,
    pub filename: String,
}

/// Read local migration files from supabase/migrations/.
pub fn list_local_migrations() -> Vec<LocalMigration> {
    let migrations_dir = Path::new("supabase/migrations");
    if !migrations_dir.exists() {
        return Vec::new();
    }

    let mut migrations = Vec::new();
    if let Ok(entries) = std::fs::read_dir(migrations_dir) {
        for entry in entries.flatten() {
            let filename = entry.file_name().to_string_lossy().to_string();
            if filename.ends_with(".sql") {
                // Format: YYYYMMDDHHMMSS_name.sql
                let stem = filename.trim_end_matches(".sql");
                let (timestamp, name) = match stem.find('_') {
                    Some(pos) => (stem[..pos].to_string(), stem[pos + 1..].to_string()),
                    None => (stem.to_string(), String::new()),
                };
                migrations.push(LocalMigration {
                    timestamp,
                    name,
                    filename,
                });
            }
        }
    }

    migrations.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
    migrations
}

/// Count local migration files.
fn count_local_migrations() -> usize {
    list_local_migrations().len()
}

/// Get remote migration status via `supabase migration list`.
pub fn remote_migration_status(runner: &dyn CommandRunner) -> ForgeResult<String> {
    let output = runner.run("supabase", &["migration", "list"])?;

    if runner.is_dry_run() {
        return Ok(String::new());
    }

    if output.success() {
        Ok(output.stdout)
    } else {
        let msg = output
            .stderr
            .lines()
            .next()
            .unwrap_or("migration list failed");
        Err(ForgeError::CommandFailed(msg.to_string()))
    }
}

/// Format local migrations for display.
pub fn format_local_migrations(migrations: &[LocalMigration]) -> String {
    if migrations.is_empty() {
        return "  No local migration files found.".to_string();
    }

    let mut lines = Vec::new();
    for m in migrations {
        if m.name.is_empty() {
            lines.push(format!("  {}  {}", m.timestamp, m.filename));
        } else {
            lines.push(format!("  {}  {}", m.timestamp, m.name));
        }
    }
    lines.join("\n")
}

// ---------------------------------------------------------------------------
// Reset
// ---------------------------------------------------------------------------

/// Reset the local Supabase database.
/// This is destructive — caller must confirm before calling.
pub fn reset_local_db(runner: &dyn CommandRunner) -> ForgeResult<String> {
    // Pre-check: project initialized?
    if !runner.is_dry_run() && !Path::new("supabase/config.toml").exists() {
        return Err(ForgeError::CommandFailed(
            "No Supabase project found. Run `supabase init` first.".to_string(),
        ));
    }

    let output = runner.run("supabase", &["db", "reset"])?;

    if runner.is_dry_run() {
        return Ok(String::new());
    }

    if output.success() {
        Ok("Local database reset successfully.".to_string())
    } else {
        let msg = output.stderr.lines().next().unwrap_or("db reset failed");
        Err(ForgeError::CommandFailed(msg.to_string()))
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
        assert!(result.cli_installed); // DryRunRunner succeeds for --version
        assert!(result.issues.iter().any(|i| i.contains("[dry-run]")));
    }

    #[test]
    fn test_summarize_status_with_services() {
        let stdout = "\
         API URL: http://localhost:54321
         DB URL: postgresql://postgres:postgres@localhost:54322/postgres
         Studio URL: http://localhost:54323
         Inbucket URL: http://localhost:54324
";
        let summary = summarize_status(stdout);
        assert!(summary.contains("API"));
        assert!(summary.contains("DB"));
        assert!(summary.contains("Studio"));
    }

    #[test]
    fn test_summarize_status_empty() {
        assert_eq!(summarize_status(""), "running");
    }

    #[test]
    fn test_list_local_migrations_no_dir() {
        // When run from the forge project root, there's no supabase/migrations/
        let migrations = list_local_migrations();
        assert!(migrations.is_empty());
    }

    #[test]
    fn test_format_local_migrations_empty() {
        let output = format_local_migrations(&[]);
        assert!(output.contains("No local migration"));
    }

    #[test]
    fn test_format_local_migrations() {
        let migrations = vec![
            LocalMigration {
                timestamp: "20240115120000".into(),
                name: "create_users".into(),
                filename: "20240115120000_create_users.sql".into(),
            },
            LocalMigration {
                timestamp: "20240120090000".into(),
                name: "add_posts".into(),
                filename: "20240120090000_add_posts.sql".into(),
            },
        ];
        let output = format_local_migrations(&migrations);
        assert!(output.contains("20240115120000"));
        assert!(output.contains("create_users"));
        assert!(output.contains("add_posts"));
    }

    #[test]
    fn test_reset_dry_run() {
        use crate::exec::DryRunRunner;
        let runner = DryRunRunner;
        let result = reset_local_db(&runner).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_remote_migration_status_dry_run() {
        use crate::exec::DryRunRunner;
        let runner = DryRunRunner;
        let result = remote_migration_status(&runner).unwrap();
        assert!(result.is_empty());
    }
}
