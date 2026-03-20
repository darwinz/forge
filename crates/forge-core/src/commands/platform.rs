//! Platform CLI diagnostics — detect installation, auth, and project link status
//! for deployment platforms (Vercel, Supabase, Netlify, Render, Appwrite).
//!
//! All checks are read-only. Auth checks use CLI commands or file existence —
//! forge never reads token contents.

use crate::exec::CommandRunner;

// ---------------------------------------------------------------------------
// Platform definition
// ---------------------------------------------------------------------------

/// A platform CLI and how to check its status.
struct PlatformDef {
    name: &'static str,
    command: &'static str,
    version_args: &'static [&'static str],
    auth_check: AuthCheck,
    link_check: Option<LinkCheck>,
}

enum AuthCheck {
    /// Run a CLI command; success = logged in.
    Command(&'static str, &'static [&'static str]),
    /// Check if a file exists at this path relative to home.
    HomeFile(&'static str),
}

struct LinkCheck {
    /// Files or directories that indicate a project is linked.
    indicators: &'static [&'static str],
}

static PLATFORMS: &[PlatformDef] = &[
    PlatformDef {
        name: "Vercel",
        command: "vercel",
        version_args: &["--version"],
        auth_check: AuthCheck::Command("vercel", &["whoami"]),
        link_check: Some(LinkCheck {
            indicators: &[".vercel/project.json"],
        }),
    },
    PlatformDef {
        name: "Supabase",
        command: "supabase",
        version_args: &["--version"],
        auth_check: AuthCheck::HomeFile(".supabase/access-token"),
        link_check: Some(LinkCheck {
            indicators: &["supabase/config.toml", "supabase/.temp/project-ref"],
        }),
    },
    PlatformDef {
        name: "Netlify",
        command: "netlify",
        version_args: &["--version"],
        auth_check: AuthCheck::HomeFile(".netlify/config.json"),
        link_check: Some(LinkCheck {
            indicators: &[".netlify/state.json"],
        }),
    },
    PlatformDef {
        name: "Render",
        command: "render",
        version_args: &["version"],
        auth_check: AuthCheck::Command("render", &["whoami"]),
        link_check: None,
    },
    PlatformDef {
        name: "Appwrite",
        command: "appwrite",
        version_args: &["--version"],
        auth_check: AuthCheck::HomeFile(".appwrite/prefs.json"),
        link_check: Some(LinkCheck {
            indicators: &["appwrite.json"],
        }),
    },
];

// ---------------------------------------------------------------------------
// Status check
// ---------------------------------------------------------------------------

/// Status of a single platform CLI.
#[derive(Debug)]
pub struct PlatformStatus {
    pub name: String,
    pub installed: bool,
    pub version: Option<String>,
}

/// Check which platform CLIs are installed and get their versions.
pub fn check_status(runner: &dyn CommandRunner) -> Vec<PlatformStatus> {
    if runner.is_dry_run() {
        return PLATFORMS
            .iter()
            .map(|p| PlatformStatus {
                name: p.name.to_string(),
                installed: false,
                version: None,
            })
            .collect();
    }

    PLATFORMS
        .iter()
        .map(|p| {
            let (installed, version) = check_installed(runner, p.command, p.version_args);
            PlatformStatus {
                name: p.name.to_string(),
                installed,
                version,
            }
        })
        .collect()
}

fn check_installed(
    runner: &dyn CommandRunner,
    command: &str,
    version_args: &[&str],
) -> (bool, Option<String>) {
    match runner.run(command, version_args) {
        Ok(output) if output.success() => {
            let version = output
                .stdout
                .lines()
                .next()
                .map(|l| l.trim().to_string())
                .filter(|v| !v.is_empty());
            (true, version)
        }
        _ => (false, None),
    }
}

// ---------------------------------------------------------------------------
// Doctor check
// ---------------------------------------------------------------------------

/// Result of a doctor check for a single platform.
#[derive(Debug)]
pub struct DoctorResult {
    pub name: String,
    pub installed: bool,
    pub version: Option<String>,
    pub authenticated: Option<bool>,
    pub auth_detail: Option<String>,
    pub project_linked: Option<bool>,
    pub link_detail: Option<String>,
}

/// Run doctor checks for all platform CLIs.
pub fn doctor(runner: &dyn CommandRunner) -> Vec<DoctorResult> {
    if runner.is_dry_run() {
        return PLATFORMS
            .iter()
            .map(|p| DoctorResult {
                name: p.name.to_string(),
                installed: false,
                version: None,
                authenticated: None,
                auth_detail: Some("[dry-run] not checked".to_string()),
                project_linked: None,
                link_detail: None,
            })
            .collect();
    }

    PLATFORMS
        .iter()
        .map(|p| check_platform(runner, p))
        .collect()
}

fn check_platform(runner: &dyn CommandRunner, platform: &PlatformDef) -> DoctorResult {
    let (installed, version) = check_installed(runner, platform.command, platform.version_args);

    if !installed {
        return DoctorResult {
            name: platform.name.to_string(),
            installed: false,
            version: None,
            authenticated: None,
            auth_detail: None,
            project_linked: None,
            link_detail: None,
        };
    }

    // Auth check
    let (authenticated, auth_detail) = check_auth(runner, &platform.auth_check);

    // Project link check
    let (project_linked, link_detail) = match &platform.link_check {
        Some(lc) => check_link(lc),
        None => (None, None),
    };

    DoctorResult {
        name: platform.name.to_string(),
        installed: true,
        version,
        authenticated: Some(authenticated),
        auth_detail,
        project_linked,
        link_detail,
    }
}

fn check_auth(runner: &dyn CommandRunner, check: &AuthCheck) -> (bool, Option<String>) {
    match check {
        AuthCheck::Command(cmd, args) => match runner.run(cmd, args) {
            Ok(output) if output.success() => {
                let user = output
                    .stdout
                    .lines()
                    .next()
                    .map(|l| l.trim().to_string())
                    .filter(|u| !u.is_empty());
                (true, user)
            }
            _ => (false, None),
        },
        AuthCheck::HomeFile(relative_path) => {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
            let path = std::path::Path::new(&home).join(relative_path);
            if path.exists() {
                (true, Some("token file present".to_string()))
            } else {
                (false, Some(format!("~/{relative_path} not found")))
            }
        }
    }
}

fn check_link(link_check: &LinkCheck) -> (Option<bool>, Option<String>) {
    let cwd = match std::env::current_dir() {
        Ok(d) => d,
        Err(_) => {
            return (
                None,
                Some("could not determine working directory".to_string()),
            )
        }
    };

    for indicator in link_check.indicators {
        if cwd.join(indicator).exists() {
            return (Some(true), Some(indicator.to_string()));
        }
    }

    (Some(false), None)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_count() {
        assert_eq!(PLATFORMS.len(), 5);
    }

    #[test]
    fn test_status_dry_run() {
        use crate::exec::DryRunRunner;
        let runner = DryRunRunner;
        let statuses = check_status(&runner);
        assert_eq!(statuses.len(), 5);
        // All should be not installed in dry-run
        assert!(statuses.iter().all(|s| !s.installed));
    }

    #[test]
    fn test_doctor_dry_run() {
        use crate::exec::DryRunRunner;
        let runner = DryRunRunner;
        let results = doctor(&runner);
        assert_eq!(results.len(), 5);
        assert!(results.iter().all(|r| !r.installed));
        // Auth detail should mention dry-run
        assert!(results
            .iter()
            .all(|r| r.auth_detail.as_deref() == Some("[dry-run] not checked")));
    }

    #[test]
    fn test_platform_names() {
        let names: Vec<&str> = PLATFORMS.iter().map(|p| p.name).collect();
        assert!(names.contains(&"Vercel"));
        assert!(names.contains(&"Supabase"));
        assert!(names.contains(&"Netlify"));
        assert!(names.contains(&"Render"));
        assert!(names.contains(&"Appwrite"));
    }
}
