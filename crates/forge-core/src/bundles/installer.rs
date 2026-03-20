//! Install planning and execution for safe package-manager-backed bundles.
//!
//! Supported sources (Phase 1):
//!   - brew formulae
//!   - brew casks
//!   - npm global packages
//!
//! Unsupported sources produce "skipped" entries in the plan.
//! Manual entries produce "manual" entries with install instructions.

use crate::exec::CommandRunner;
use crate::error::ForgeResult;
use super::inventory::{BundleInventory, PackageState};
use super::registry::BundleManifest;

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
        self.actions.iter().filter(|a| a.kind == ActionKind::Install).collect()
    }

    pub fn already_installed(&self) -> Vec<&InstallAction> {
        self.actions.iter().filter(|a| a.kind == ActionKind::AlreadyInstalled).collect()
    }

    pub fn skipped(&self) -> Vec<&InstallAction> {
        self.actions.iter().filter(|a| matches!(a.kind, ActionKind::Skipped { .. })).collect()
    }

    pub fn manual(&self) -> Vec<&InstallAction> {
        self.actions.iter().filter(|a| matches!(a.kind, ActionKind::Manual { .. })).collect()
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
                // (brew install / npm install -g are idempotent if already present)
                ("brew", PackageState::Missing) |
                ("brew", PackageState::Unknown) |
                ("brew-cask", PackageState::Missing) |
                ("brew-cask", PackageState::Unknown) |
                ("npm", PackageState::Missing) |
                ("npm", PackageState::Unknown) => {
                    InstallAction {
                        package: pkg.name.clone(),
                        source: source_str,
                        kind: ActionKind::Install,
                    }
                }
                (_, PackageState::Installed) => {
                    InstallAction {
                        package: pkg.name.clone(),
                        source: source_str,
                        kind: ActionKind::AlreadyInstalled,
                    }
                }
                // Manual entries
                ("manual", _) => {
                    // Look up the manual entry for instructions
                    let manual_info = manifest.manual.as_ref()
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
                // Unsupported sources (go, gem, pipx, etc.) — skip for now
                (_, PackageState::Missing) | (_, PackageState::Unknown) | (_, PackageState::Unavailable) => {
                    InstallAction {
                        package: pkg.name.clone(),
                        source: source_str.clone(),
                        kind: ActionKind::Skipped {
                            reason: format!("{source_str} installation not yet supported"),
                        },
                    }
                }
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
pub fn execute_plan(
    plan: &InstallPlan,
    runner: &dyn CommandRunner,
) -> Vec<InstallResult> {
    let mut results = Vec::new();

    // Batch by source for efficiency
    let brew_formulae: Vec<&str> = plan.to_install().iter()
        .filter(|a| a.source == "brew")
        .map(|a| a.package.as_str())
        .collect();

    let brew_casks: Vec<&str> = plan.to_install().iter()
        .filter(|a| a.source == "brew-cask")
        .map(|a| a.package.as_str())
        .collect();

    let npm_packages: Vec<&str> = plan.to_install().iter()
        .filter(|a| a.source == "npm")
        .map(|a| a.package.as_str())
        .collect();

    // Install brew formulae
    if !brew_formulae.is_empty() {
        let mut args = vec!["install"];
        args.extend(&brew_formulae);
        match runner.run("brew", &args) {
            Ok(output) if output.success() => {
                for pkg in &brew_formulae {
                    results.push(InstallResult {
                        package: pkg.to_string(),
                        source: "brew".to_string(),
                        success: true,
                        message: "installed".to_string(),
                    });
                }
            }
            Ok(output) => {
                // Partial failure — report stderr
                let msg = if output.stderr.is_empty() {
                    "install failed".to_string()
                } else {
                    output.stderr.lines().next().unwrap_or("install failed").to_string()
                };
                for pkg in &brew_formulae {
                    results.push(InstallResult {
                        package: pkg.to_string(),
                        source: "brew".to_string(),
                        success: false,
                        message: msg.clone(),
                    });
                }
            }
            Err(e) => {
                for pkg in &brew_formulae {
                    results.push(InstallResult {
                        package: pkg.to_string(),
                        source: "brew".to_string(),
                        success: false,
                        message: format!("{e}"),
                    });
                }
            }
        }
    }

    // Install brew casks
    if !brew_casks.is_empty() {
        let mut args = vec!["install", "--cask"];
        args.extend(&brew_casks);
        match runner.run("brew", &args) {
            Ok(output) if output.success() => {
                for pkg in &brew_casks {
                    results.push(InstallResult {
                        package: pkg.to_string(),
                        source: "brew-cask".to_string(),
                        success: true,
                        message: "installed".to_string(),
                    });
                }
            }
            Ok(output) => {
                let msg = output.stderr.lines().next().unwrap_or("install failed").to_string();
                for pkg in &brew_casks {
                    results.push(InstallResult {
                        package: pkg.to_string(),
                        source: "brew-cask".to_string(),
                        success: false,
                        message: msg.clone(),
                    });
                }
            }
            Err(e) => {
                for pkg in &brew_casks {
                    results.push(InstallResult {
                        package: pkg.to_string(),
                        source: "brew-cask".to_string(),
                        success: false,
                        message: format!("{e}"),
                    });
                }
            }
        }
    }

    // Install npm packages
    if !npm_packages.is_empty() {
        let mut args = vec!["install", "-g"];
        args.extend(&npm_packages);
        match runner.run("npm", &args) {
            Ok(output) if output.success() => {
                for pkg in &npm_packages {
                    results.push(InstallResult {
                        package: pkg.to_string(),
                        source: "npm".to_string(),
                        success: true,
                        message: "installed".to_string(),
                    });
                }
            }
            Ok(output) => {
                let msg = output.stderr.lines().next().unwrap_or("install failed").to_string();
                for pkg in &npm_packages {
                    results.push(InstallResult {
                        package: pkg.to_string(),
                        source: "npm".to_string(),
                        success: false,
                        message: msg.clone(),
                    });
                }
            }
            Err(e) => {
                for pkg in &npm_packages {
                    results.push(InstallResult {
                        package: pkg.to_string(),
                        source: "npm".to_string(),
                        success: false,
                        message: format!("{e}"),
                    });
                }
            }
        }
    }

    results
}

// ---------------------------------------------------------------------------
// Profile loading
// ---------------------------------------------------------------------------

/// Load default bundle names from a profile TOML file.
pub fn load_default_profile(profile_path: &std::path::Path) -> ForgeResult<Vec<String>> {
    if !profile_path.exists() {
        return Ok(Vec::new());
    }
    let content = std::fs::read_to_string(profile_path)
        .map_err(|e| crate::ForgeError::FileRead {
            path: profile_path.to_path_buf(),
            source: e,
        })?;

    #[derive(serde::Deserialize)]
    struct Profile {
        bundles: Vec<String>,
    }

    let profile: Profile = toml::from_str(&content)
        .map_err(|e| crate::ForgeError::TomlParse {
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
                    kind: ActionKind::Skipped {
                        reason: "go installation not yet supported".into(),
                    },
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

        assert_eq!(plan.to_install().len(), 1);
        assert_eq!(plan.to_install()[0].package, "bat");
        assert_eq!(plan.already_installed().len(), 1);
        assert_eq!(plan.skipped().len(), 1);
        assert_eq!(plan.manual().len(), 1);
        assert!(plan.has_work());
    }

    #[test]
    fn test_install_plan_no_work() {
        let plan = InstallPlan {
            actions: vec![
                InstallAction {
                    package: "jq".into(),
                    source: "brew".into(),
                    kind: ActionKind::AlreadyInstalled,
                },
            ],
        };
        assert!(!plan.has_work());
    }
}
