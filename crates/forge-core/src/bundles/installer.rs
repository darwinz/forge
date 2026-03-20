//! Install planning and execution for safe package-manager-backed bundles.
//!
//! Supported sources:
//!   - brew formulae (on PATH via brew)
//!   - brew casks (on PATH via brew)
//!   - npm global packages (on PATH via npm prefix)
//!   - go tools (`go install` — binary goes to $GOPATH/bin, needs PATH setup)
//!   - gem packages (`gem install --user-install` — needs PATH setup, no sudo)
//!   - uv tools (`uv tool install` — isolated venvs, binaries in ~/.local/bin)
//!
//! Each package is installed individually for accurate per-package error reporting.
//! After execution, call `path_hints_for_results()` to surface PATH guidance for
//! sources like go and gem whose binaries may not be on the user's default PATH.
//!
//! Unsupported sources produce "skipped" entries in the plan.
//! Manual entries produce "manual" entries with install instructions.

use super::inventory::{BundleInventory, PackageState};
use super::registry::BundleManifest;
use crate::error::ForgeResult;
use crate::exec::CommandRunner;

// ---------------------------------------------------------------------------
// Install plan
// ---------------------------------------------------------------------------

/// A single action in an install plan.
#[derive(Debug, Clone)]
pub struct InstallAction {
    pub package: String,
    pub source: String,
    pub kind: ActionKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionKind {
    /// Will be installed via a supported package manager.
    Install,
    /// Already installed — no action needed.
    AlreadyInstalled,
    /// Source is not yet supported for installation.
    Skipped { reason: String },
    /// Manual install — show instructions to the user.
    Manual {
        instructions: String,
        notes: Option<String>,
        detected: bool,
    },
}

/// A complete install plan for one or more bundles.
#[derive(Debug)]
pub struct InstallPlan {
    pub actions: Vec<InstallAction>,
}

impl InstallPlan {
    pub fn to_install(&self) -> Vec<&InstallAction> {
        self.actions
            .iter()
            .filter(|a| a.kind == ActionKind::Install)
            .collect()
    }

    pub fn already_installed(&self) -> Vec<&InstallAction> {
        self.actions
            .iter()
            .filter(|a| a.kind == ActionKind::AlreadyInstalled)
            .collect()
    }

    pub fn skipped(&self) -> Vec<&InstallAction> {
        self.actions
            .iter()
            .filter(|a| matches!(a.kind, ActionKind::Skipped { .. }))
            .collect()
    }

    pub fn manual(&self) -> Vec<&InstallAction> {
        self.actions
            .iter()
            .filter(|a| matches!(a.kind, ActionKind::Manual { .. }))
            .collect()
    }

    pub fn has_work(&self) -> bool {
        self.actions.iter().any(|a| a.kind == ActionKind::Install)
    }
}

// ---------------------------------------------------------------------------
// Planning
// ---------------------------------------------------------------------------

/// Build an install plan by checking current state against the manifest.
pub fn plan_install(
    manifests: &[&BundleManifest],
    runner: &dyn CommandRunner,
) -> ForgeResult<InstallPlan> {
    let mut actions = Vec::new();

    for manifest in manifests {
        let report = BundleInventory::check(manifest, runner)?;

        // Walk the report to classify each package
        for pkg in &report.packages {
            let source_str = pkg.source_label();
            let action = match (&source_str[..], &pkg.state) {
                // Supported sources — Missing or Unknown both plan for install
                // (brew install / npm install -g / go install / gem install are idempotent)
                ("brew", PackageState::Missing)
                | ("brew", PackageState::Unknown)
                | ("brew-cask", PackageState::Missing)
                | ("brew-cask", PackageState::Unknown)
                | ("npm", PackageState::Missing)
                | ("npm", PackageState::Unknown)
                | ("go", PackageState::Missing)
                | ("go", PackageState::Unknown)
                | ("gem", PackageState::Missing)
                | ("gem", PackageState::Unknown)
                | ("uv-tool", PackageState::Missing)
                | ("uv-tool", PackageState::Unknown) => InstallAction {
                    package: pkg.name.clone(),
                    source: source_str,
                    kind: ActionKind::Install,
                },
                (_, PackageState::Installed) => InstallAction {
                    package: pkg.name.clone(),
                    source: source_str,
                    kind: ActionKind::AlreadyInstalled,
                },
                // Manual entries
                ("manual", _) => {
                    // Look up the manual entry for instructions
                    let manual_info = manifest
                        .manual
                        .as_ref()
                        .and_then(|ms| ms.iter().find(|m| m.name == pkg.name));
                    match manual_info {
                        Some(entry) => InstallAction {
                            package: pkg.name.clone(),
                            source: source_str,
                            kind: ActionKind::Manual {
                                instructions: entry.install_instructions.clone(),
                                notes: entry.notes.clone(),
                                detected: pkg.state.is_installed(),
                            },
                        },
                        None => InstallAction {
                            package: pkg.name.clone(),
                            source: source_str,
                            kind: ActionKind::Skipped {
                                reason: "no install instructions".to_string(),
                            },
                        },
                    }
                }
                // Unsupported sources (pipx, pnpm, bun, composer) — skip for now
                (_, PackageState::Missing)
                | (_, PackageState::Unknown)
                | (_, PackageState::Unavailable) => InstallAction {
                    package: pkg.name.clone(),
                    source: source_str.clone(),
                    kind: ActionKind::Skipped {
                        reason: format!("{source_str} installation not yet supported"),
                    },
                },
            };
            actions.push(action);
        }
    }

    Ok(InstallPlan { actions })
}

// ---------------------------------------------------------------------------
// Execution
// ---------------------------------------------------------------------------

/// Result of executing a single install action.
#[derive(Debug)]
pub struct InstallResult {
    pub package: String,
    pub source: String,
    pub success: bool,
    pub message: String,
}

/// Execute the install actions from a plan.
/// Only runs actions with `ActionKind::Install`.
///
/// Each package is installed individually so that a single failure does not
/// misreport the entire batch. This is slightly slower than batching but
/// produces accurate per-package success/failure reporting.
pub fn execute_plan(plan: &InstallPlan, runner: &dyn CommandRunner) -> Vec<InstallResult> {
    let mut results = Vec::new();

    for action in plan.to_install() {
        let result = match action.source.as_str() {
            "brew" => install_one(
                runner,
                "brew",
                &["install", &action.package],
                &action.package,
                "brew",
            ),
            "brew-cask" => install_one(
                runner,
                "brew",
                &["install", "--cask", &action.package],
                &action.package,
                "brew-cask",
            ),
            "npm" => install_one(
                runner,
                "npm",
                &["install", "-g", &action.package],
                &action.package,
                "npm",
            ),
            "go" => install_one(
                runner,
                "go",
                &["install", &action.package],
                &action.package,
                "go",
            ),
            "gem" => install_one(
                runner,
                "gem",
                &["install", "--user-install", &action.package],
                &action.package,
                "gem",
            ),
            "uv-tool" => install_one(
                runner,
                "uv",
                &["tool", "install", &action.package],
                &action.package,
                "uv-tool",
            ),
            other => InstallResult {
                package: action.package.clone(),
                source: other.to_string(),
                success: false,
                message: format!("{other} execution not supported"),
            },
        };
        results.push(result);
    }

    results
}

/// Run a single package install command and return its result.
fn install_one(
    runner: &dyn CommandRunner,
    program: &str,
    args: &[&str],
    package: &str,
    source_label: &str,
) -> InstallResult {
    match runner.run(program, args) {
        Ok(output) if output.success() || runner.is_dry_run() => InstallResult {
            package: package.to_string(),
            source: source_label.to_string(),
            success: true,
            message: "installed".to_string(),
        },
        Ok(output) => {
            let msg = if output.stderr.is_empty() {
                "install failed".to_string()
            } else {
                output
                    .stderr
                    .lines()
                    .next()
                    .unwrap_or("install failed")
                    .to_string()
            };
            InstallResult {
                package: package.to_string(),
                source: source_label.to_string(),
                success: false,
                message: msg,
            }
        }
        Err(e) => InstallResult {
            package: package.to_string(),
            source: source_label.to_string(),
            success: false,
            message: format!("{e}"),
        },
    }
}

/// Post-install PATH hints for sources whose binaries may not be on PATH.
///
/// Returns `Some(hint)` if the source typically needs PATH configuration,
/// `None` if binaries should be on PATH already (e.g. brew, npm).
pub fn path_hint(source: &str) -> Option<&'static str> {
    match source {
        "go" => Some("Go binaries are installed to $GOPATH/bin (default: ~/go/bin). Ensure it is on your PATH."),
        "gem" => Some("Gem binaries from --user-install go to ~/.gem/ruby/<version>/bin. Ensure it is on your PATH."),
        "uv-tool" => Some("uv tool binaries are installed to ~/.local/bin. Ensure it is on your PATH."),
        _ => None,
    }
}

/// Returns all unique sources that have path hints, given a list of results.
pub fn path_hints_for_results(results: &[InstallResult]) -> Vec<&'static str> {
    let mut seen = std::collections::HashSet::new();
    let mut hints = Vec::new();
    for r in results {
        if r.success {
            if let Some(hint) = path_hint(&r.source) {
                if seen.insert(&r.source) {
                    hints.push(hint);
                }
            }
        }
    }
    hints
}

// ---------------------------------------------------------------------------
// Profile loading
// ---------------------------------------------------------------------------

/// Load default bundle names from a profile TOML file.
pub fn load_default_profile(profile_path: &std::path::Path) -> ForgeResult<Vec<String>> {
    if !profile_path.exists() {
        return Ok(Vec::new());
    }
    let content =
        std::fs::read_to_string(profile_path).map_err(|e| crate::ForgeError::FileRead {
            path: profile_path.to_path_buf(),
            source: e,
        })?;

    #[derive(serde::Deserialize)]
    struct Profile {
        bundles: Vec<String>,
    }

    let profile: Profile = toml::from_str(&content).map_err(|e| crate::ForgeError::TomlParse {
        path: profile_path.to_path_buf(),
        source: e,
    })?;
    Ok(profile.bundles)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_kind_equality() {
        assert_eq!(ActionKind::Install, ActionKind::Install);
        assert_eq!(ActionKind::AlreadyInstalled, ActionKind::AlreadyInstalled);
        assert_ne!(ActionKind::Install, ActionKind::AlreadyInstalled);
    }

    #[test]
    fn test_install_plan_filters() {
        let plan = InstallPlan {
            actions: vec![
                InstallAction {
                    package: "bat".into(),
                    source: "brew".into(),
                    kind: ActionKind::Install,
                },
                InstallAction {
                    package: "jq".into(),
                    source: "brew".into(),
                    kind: ActionKind::AlreadyInstalled,
                },
                InstallAction {
                    package: "gopls".into(),
                    source: "go".into(),
                    kind: ActionKind::Install,
                },
                InstallAction {
                    package: "claude".into(),
                    source: "manual".into(),
                    kind: ActionKind::Manual {
                        instructions: "install from anthropic.com".into(),
                        notes: None,
                        detected: true,
                    },
                },
            ],
        };

        assert_eq!(plan.to_install().len(), 2);
        assert_eq!(plan.to_install()[0].package, "bat");
        assert_eq!(plan.to_install()[1].package, "gopls");
        assert_eq!(plan.already_installed().len(), 1);
        assert_eq!(plan.skipped().len(), 0);
        assert_eq!(plan.manual().len(), 1);
        assert!(plan.has_work());
    }

    #[test]
    fn test_install_plan_no_work() {
        let plan = InstallPlan {
            actions: vec![InstallAction {
                package: "jq".into(),
                source: "brew".into(),
                kind: ActionKind::AlreadyInstalled,
            }],
        };
        assert!(!plan.has_work());
    }

    #[test]
    fn test_install_plan_with_go_and_gem() {
        let plan = InstallPlan {
            actions: vec![
                InstallAction {
                    package: "golang.org/x/tools/gopls@latest".into(),
                    source: "go".into(),
                    kind: ActionKind::Install,
                },
                InstallAction {
                    package: "bundler".into(),
                    source: "gem".into(),
                    kind: ActionKind::Install,
                },
                InstallAction {
                    package: "bat".into(),
                    source: "brew".into(),
                    kind: ActionKind::AlreadyInstalled,
                },
            ],
        };

        let to_install = plan.to_install();
        assert_eq!(to_install.len(), 2);
        assert!(to_install.iter().any(|a| a.source == "go"));
        assert!(to_install.iter().any(|a| a.source == "gem"));
        assert!(plan.has_work());
    }

    #[test]
    fn test_install_plan_skipped_sources() {
        let plan = InstallPlan {
            actions: vec![
                InstallAction {
                    package: "some-pkg".into(),
                    source: "pipx".into(),
                    kind: ActionKind::Skipped {
                        reason: "pipx installation not yet supported".into(),
                    },
                },
                InstallAction {
                    package: "ruff".into(),
                    source: "uv-tool".into(),
                    kind: ActionKind::Skipped {
                        reason: "uv-tool installation not yet supported".into(),
                    },
                },
            ],
        };

        assert_eq!(plan.skipped().len(), 2);
        assert!(!plan.has_work());
    }

    #[test]
    fn test_execute_plan_dry_run() {
        use crate::exec::DryRunRunner;

        let plan = InstallPlan {
            actions: vec![
                InstallAction {
                    package: "bat".into(),
                    source: "brew".into(),
                    kind: ActionKind::Install,
                },
                InstallAction {
                    package: "golang.org/x/tools/gopls@latest".into(),
                    source: "go".into(),
                    kind: ActionKind::Install,
                },
                InstallAction {
                    package: "bundler".into(),
                    source: "gem".into(),
                    kind: ActionKind::Install,
                },
            ],
        };

        let runner = DryRunRunner;
        let results = execute_plan(&plan, &runner);

        // All should succeed in dry-run, one result per package
        assert_eq!(results.len(), 3);
        assert!(results.iter().all(|r| r.success));
        assert!(results.iter().any(|r| r.source == "brew"));
        assert!(results.iter().any(|r| r.source == "go"));
        assert!(results.iter().any(|r| r.source == "gem"));
    }

    #[test]
    fn test_execute_plan_per_package_results() {
        use crate::exec::DryRunRunner;

        // Verify that each package gets its own result, not batch-grouped
        let plan = InstallPlan {
            actions: vec![
                InstallAction {
                    package: "bat".into(),
                    source: "brew".into(),
                    kind: ActionKind::Install,
                },
                InstallAction {
                    package: "jq".into(),
                    source: "brew".into(),
                    kind: ActionKind::Install,
                },
                InstallAction {
                    package: "typescript".into(),
                    source: "npm".into(),
                    kind: ActionKind::Install,
                },
            ],
        };

        let runner = DryRunRunner;
        let results = execute_plan(&plan, &runner);

        // Each package gets its own result
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].package, "bat");
        assert_eq!(results[1].package, "jq");
        assert_eq!(results[2].package, "typescript");
    }

    #[test]
    fn test_execute_plan_skips_non_install() {
        use crate::exec::DryRunRunner;

        let plan = InstallPlan {
            actions: vec![
                InstallAction {
                    package: "jq".into(),
                    source: "brew".into(),
                    kind: ActionKind::AlreadyInstalled,
                },
                InstallAction {
                    package: "claude".into(),
                    source: "manual".into(),
                    kind: ActionKind::Manual {
                        instructions: "install manually".into(),
                        notes: None,
                        detected: false,
                    },
                },
            ],
        };

        let runner = DryRunRunner;
        let results = execute_plan(&plan, &runner);

        // No Install actions → no results
        assert!(results.is_empty());
    }

    #[test]
    fn test_path_hints() {
        assert!(path_hint("go").is_some());
        assert!(path_hint("gem").is_some());
        assert!(path_hint("brew").is_none());
        assert!(path_hint("npm").is_none());
        assert!(path_hint("brew-cask").is_none());
    }

    #[test]
    fn test_path_hints_for_results_dedup() {
        let results = vec![
            InstallResult {
                package: "gopls".into(),
                source: "go".into(),
                success: true,
                message: "installed".into(),
            },
            InstallResult {
                package: "dlv".into(),
                source: "go".into(),
                success: true,
                message: "installed".into(),
            },
            InstallResult {
                package: "bundler".into(),
                source: "gem".into(),
                success: true,
                message: "installed".into(),
            },
            InstallResult {
                package: "bat".into(),
                source: "brew".into(),
                success: true,
                message: "installed".into(),
            },
        ];

        let hints = path_hints_for_results(&results);
        // go and gem each appear once, brew has no hint
        assert_eq!(hints.len(), 2);
    }

    #[test]
    fn test_path_hints_skips_failures() {
        let results = vec![InstallResult {
            package: "gopls".into(),
            source: "go".into(),
            success: false,
            message: "go not found".into(),
        }];

        let hints = path_hints_for_results(&results);
        // Failed installs should not produce path hints
        assert!(hints.is_empty());
    }

    #[test]
    fn test_load_default_profile() {
        use std::io::Write;

        let dir = std::env::temp_dir().join("forge_test_profile");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("profile.toml");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, "bundles = [\"core\", \"git\", \"node\"]").unwrap();

        let bundles = load_default_profile(&path).unwrap();
        assert_eq!(bundles, vec!["core", "git", "node"]);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_load_default_profile_missing_file() {
        let path = std::path::Path::new("/tmp/forge_test_nonexistent_profile.toml");
        let bundles = load_default_profile(path).unwrap();
        assert!(bundles.is_empty());
    }
}
