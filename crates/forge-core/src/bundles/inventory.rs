use crate::exec::CommandRunner;
use crate::error::ForgeResult;
use super::registry::BundleManifest;
use super::sources::{
    self, DetectedPackage, InstallSource, PackageManagerKind, SourceCategory,
};

// ---------------------------------------------------------------------------
// Package state — replaces the old bool
// ---------------------------------------------------------------------------

/// Detection state for a package.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageState {
    /// Package detected as installed.
    Installed,
    /// Package not found by the relevant manager.
    Missing,
    /// Status cannot be determined (dry-run mode or manager unavailable).
    Unknown,
    /// The package manager itself is not installed.
    Unavailable,
}

impl PackageState {
    pub fn is_installed(&self) -> bool {
        *self == PackageState::Installed
    }

    /// Label for display.
    pub fn label(&self) -> &'static str {
        match self {
            PackageState::Installed => "installed",
            PackageState::Missing => "missing",
            PackageState::Unknown => "unknown",
            PackageState::Unavailable => "unavailable",
        }
    }
}

impl std::fmt::Display for PackageState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

// ---------------------------------------------------------------------------
// PackageStatus / BundleReport
// ---------------------------------------------------------------------------

/// Status of a single package within a bundle check.
#[derive(Debug)]
pub struct PackageStatus {
    pub name: String,
    pub source: InstallSource,
    /// Version if detected.
    pub version: Option<String>,
    pub state: PackageState,
}

impl PackageStatus {
    /// Short label for display.
    pub fn source_label(&self) -> String {
        self.source.to_string()
    }

    pub fn category(&self) -> SourceCategory {
        self.source.category()
    }
}

/// Inventory report for a bundle.
#[derive(Debug)]
pub struct BundleReport {
    pub bundle_name: String,
    pub tier: String,
    pub packages: Vec<PackageStatus>,
}

impl BundleReport {
    pub fn installed_count(&self) -> usize {
        self.packages.iter().filter(|p| p.state.is_installed()).count()
    }

    pub fn total_count(&self) -> usize {
        self.packages.len()
    }

    pub fn missing_packages(&self) -> Vec<&PackageStatus> {
        self.packages.iter().filter(|p| p.state == PackageState::Missing).collect()
    }

    /// Summary label for the bundle.
    pub fn summary_label(&self) -> &'static str {
        let has_unknown = self.packages.iter().any(|p| {
            p.state == PackageState::Unknown || p.state == PackageState::Unavailable
        });
        if self.installed_count() == self.total_count() {
            "all installed"
        } else if has_unknown {
            "partially checked"
        } else {
            "has missing packages"
        }
    }
}

// ---------------------------------------------------------------------------
// Inventory result from a manager query — distinguishes "empty list" from
// "manager not available"
// ---------------------------------------------------------------------------

enum ManagerResult {
    /// Manager ran successfully and returned these package names.
    Available(Vec<String>),
    /// Manager is not installed or failed to run.
    Unavailable,
}

impl ManagerResult {
    fn lookup_state(&self, pkg_name: &str) -> PackageState {
        match self {
            ManagerResult::Available(installed) => {
                if installed.iter().any(|n| n == pkg_name) {
                    PackageState::Installed
                } else {
                    PackageState::Missing
                }
            }
            ManagerResult::Unavailable => PackageState::Unavailable,
        }
    }
}

/// Like ManagerResult but for detection-adapter outputs.
enum DetectionResult {
    Available(Vec<DetectedPackage>),
    Unavailable,
}

impl DetectionResult {
    fn lookup_state(&self, pkg_name: &str) -> (PackageState, Option<String>) {
        match self {
            DetectionResult::Available(detected) => {
                if let Some(d) = detected.iter().find(|d| d.name == *pkg_name) {
                    (PackageState::Installed, d.version.clone())
                } else {
                    (PackageState::Missing, None)
                }
            }
            DetectionResult::Unavailable => (PackageState::Unavailable, None),
        }
    }
}

// ---------------------------------------------------------------------------
// BundleInventory
// ---------------------------------------------------------------------------

/// Bundle inventory checker.
pub struct BundleInventory;

impl BundleInventory {
    /// Check installed status of all packages in a bundle.
    pub fn check(
        manifest: &BundleManifest,
        runner: &dyn CommandRunner,
    ) -> ForgeResult<BundleReport> {
        let mut packages = Vec::new();
        let dry_run = runner.is_dry_run();

        // --- brew ---
        if let Some(ref brew) = manifest.brew {
            let installed = query_brew(runner, false);
            for formula in &brew.formulae {
                packages.push(PackageStatus {
                    name: formula.clone(),
                    source: InstallSource::PackageManager(PackageManagerKind::Brew),
                    version: None,
                    state: if dry_run { PackageState::Unknown } else { installed.lookup_state(formula) },
                });
            }
            let installed_casks = query_brew(runner, true);
            for cask in &brew.casks {
                packages.push(PackageStatus {
                    name: cask.clone(),
                    source: InstallSource::PackageManager(PackageManagerKind::BrewCask),
                    version: None,
                    state: if dry_run { PackageState::Unknown } else { installed_casks.lookup_state(cask) },
                });
            }
        }

        // --- npm ---
        if let Some(ref npm) = manifest.npm {
            let installed = query_npm(runner);
            for pkg in &npm.packages {
                packages.push(PackageStatus {
                    name: pkg.clone(),
                    source: InstallSource::PackageManager(PackageManagerKind::Npm),
                    version: None,
                    state: if dry_run { PackageState::Unknown } else { installed.lookup_state(pkg) },
                });
            }
        }

        // --- pnpm ---
        if let Some(ref pnpm) = manifest.pnpm {
            let detected = query_detection(runner, sources::detect_pnpm);
            check_against_detection(&pnpm.packages, &detected, PackageManagerKind::Pnpm, dry_run, &mut packages);
        }

        // --- bun ---
        if let Some(ref bun) = manifest.bun {
            let detected = query_detection(runner, sources::detect_bun);
            check_against_detection(&bun.packages, &detected, PackageManagerKind::Bun, dry_run, &mut packages);
        }

        // --- go ---
        if let Some(ref go) = manifest.go {
            for tool in &go.tools {
                let binary = tool
                    .split('/')
                    .last()
                    .unwrap_or(tool)
                    .split('@')
                    .next()
                    .unwrap_or(tool);
                let state = if dry_run {
                    PackageState::Unknown
                } else {
                    command_state(runner, binary)
                };
                packages.push(PackageStatus {
                    name: tool.clone(),
                    source: InstallSource::PackageManager(PackageManagerKind::Go),
                    version: None,
                    state,
                });
            }
        }

        // --- gem ---
        if let Some(ref gem) = manifest.gem {
            let installed = query_gem(runner);
            for pkg in &gem.packages {
                packages.push(PackageStatus {
                    name: pkg.clone(),
                    source: InstallSource::PackageManager(PackageManagerKind::Gem),
                    version: None,
                    state: if dry_run { PackageState::Unknown } else { installed.lookup_state(pkg) },
                });
            }
        }

        // --- pipx ---
        if let Some(ref pipx) = manifest.pipx {
            let detected = query_detection(runner, sources::detect_pipx);
            check_against_detection(&pipx.packages, &detected, PackageManagerKind::Pipx, dry_run, &mut packages);
        }

        // --- uv tool ---
        if let Some(ref uv) = manifest.uv_tool {
            let detected = query_detection(runner, sources::detect_uv_tool);
            check_against_detection(&uv.tools, &detected, PackageManagerKind::UvTool, dry_run, &mut packages);
        }

        // --- composer ---
        if let Some(ref composer) = manifest.composer {
            let detected = query_detection(runner, sources::detect_composer);
            check_against_detection(
                &composer.packages,
                &detected,
                PackageManagerKind::Composer,
                dry_run,
                &mut packages,
            );
        }

        // --- manual ---
        if let Some(ref manuals) = manifest.manual {
            for entry in manuals {
                let parts: Vec<&str> = entry.check_command.split_whitespace().collect();
                let state = if dry_run {
                    PackageState::Unknown
                } else if let Some(cmd) = parts.first() {
                    command_state(runner, cmd)
                } else {
                    PackageState::Unknown
                };
                packages.push(PackageStatus {
                    name: entry.name.clone(),
                    source: InstallSource::Manual,
                    version: None,
                    state,
                });
            }
        }

        Ok(BundleReport {
            bundle_name: manifest.bundle.name.clone(),
            tier: manifest.bundle.tier.clone(),
            packages,
        })
    }
}

// ---------------------------------------------------------------------------
// Manager queries — return ManagerResult to distinguish unavailable
// ---------------------------------------------------------------------------

fn query_brew(runner: &dyn CommandRunner, cask: bool) -> ManagerResult {
    if runner.is_dry_run() {
        return ManagerResult::Unavailable;
    }
    let args: Vec<&str> = if cask {
        vec!["list", "--cask", "-1"]
    } else {
        vec!["list", "--formula", "-1"]
    };
    match runner.run("brew", &args) {
        Ok(output) if output.success() => ManagerResult::Available(
            output.stdout.lines().map(|l| l.trim().to_string()).collect(),
        ),
        _ => ManagerResult::Unavailable,
    }
}

fn query_npm(runner: &dyn CommandRunner) -> ManagerResult {
    if runner.is_dry_run() {
        return ManagerResult::Unavailable;
    }
    match runner.run("npm", &["list", "-g", "--depth=0", "--parseable"]) {
        Ok(output) if output.success() => {
            ManagerResult::Available(parse_npm_parseable(&output.stdout))
        }
        _ => ManagerResult::Unavailable,
    }
}

/// Parse `npm list -g --depth=0 --parseable` output, preserving scoped package names.
///
/// Output lines look like:
///   /usr/local/lib/node_modules/typescript
///   /usr/local/lib/node_modules/@openai/codex
///
/// For scoped packages we need to capture `@scope/name`, not just the last segment.
pub fn parse_npm_parseable(stdout: &str) -> Vec<String> {
    stdout
        .lines()
        .filter(|l| !l.trim().is_empty() && l.contains("node_modules/"))
        .filter_map(|line| {
            // Split at "node_modules/" and take what follows
            let after_nm = line.rsplit("node_modules/").next()?;
            if after_nm.is_empty() {
                return None;
            }
            Some(after_nm.to_string())
        })
        .collect()
}

fn query_gem(runner: &dyn CommandRunner) -> ManagerResult {
    if runner.is_dry_run() {
        return ManagerResult::Unavailable;
    }
    match runner.run("gem", &["list", "--no-details"]) {
        Ok(output) if output.success() => ManagerResult::Available(
            output.stdout.lines()
                .filter_map(|l| l.split_whitespace().next())
                .map(|s| s.to_string())
                .collect(),
        ),
        _ => ManagerResult::Unavailable,
    }
}

fn query_detection(
    runner: &dyn CommandRunner,
    detect_fn: fn(&dyn CommandRunner) -> ForgeResult<Vec<DetectedPackage>>,
) -> DetectionResult {
    if runner.is_dry_run() {
        return DetectionResult::Unavailable;
    }
    match detect_fn(runner) {
        Ok(pkgs) => DetectionResult::Available(pkgs),
        Err(_) => DetectionResult::Unavailable,
    }
}

fn check_against_detection(
    expected: &[String],
    detected: &DetectionResult,
    kind: PackageManagerKind,
    dry_run: bool,
    out: &mut Vec<PackageStatus>,
) {
    for pkg_name in expected {
        let (state, version) = if dry_run {
            (PackageState::Unknown, None)
        } else {
            detected.lookup_state(pkg_name)
        };
        out.push(PackageStatus {
            name: pkg_name.clone(),
            source: InstallSource::PackageManager(kind),
            version,
            state,
        });
    }
}

fn command_state(runner: &dyn CommandRunner, cmd: &str) -> PackageState {
    match runner.run("which", &[cmd]) {
        Ok(output) if output.success() => PackageState::Installed,
        _ => PackageState::Missing,
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_npm_parseable_unscoped() {
        let input = "\
/usr/local/lib/node_modules/npm
/usr/local/lib/node_modules/typescript
/usr/local/lib/node_modules/eslint
";
        let pkgs = parse_npm_parseable(input);
        assert_eq!(pkgs, vec!["npm", "typescript", "eslint"]);
    }

    #[test]
    fn test_parse_npm_parseable_scoped() {
        let input = "\
/usr/local/lib/node_modules/npm
/usr/local/lib/node_modules/@openai/codex
/usr/local/lib/node_modules/@google/gemini-cli
/usr/local/lib/node_modules/typescript
";
        let pkgs = parse_npm_parseable(input);
        assert_eq!(
            pkgs,
            vec!["npm", "@openai/codex", "@google/gemini-cli", "typescript"]
        );
    }

    #[test]
    fn test_parse_npm_parseable_root_line() {
        // First line is often just the root path without a package
        let input = "/usr/local/lib\n/usr/local/lib/node_modules/typescript\n";
        let pkgs = parse_npm_parseable(input);
        assert_eq!(pkgs, vec!["typescript"]);
    }

    #[test]
    fn test_parse_npm_parseable_empty() {
        assert!(parse_npm_parseable("").is_empty());
        assert!(parse_npm_parseable("   \n  \n").is_empty());
    }

    #[test]
    fn test_package_state_labels() {
        assert_eq!(PackageState::Installed.label(), "installed");
        assert_eq!(PackageState::Missing.label(), "missing");
        assert_eq!(PackageState::Unknown.label(), "unknown");
        assert_eq!(PackageState::Unavailable.label(), "unavailable");
    }

    #[test]
    fn test_manager_result_lookup() {
        let avail = ManagerResult::Available(vec![
            "foo".to_string(),
            "bar".to_string(),
        ]);
        assert_eq!(avail.lookup_state("foo"), PackageState::Installed);
        assert_eq!(avail.lookup_state("baz"), PackageState::Missing);

        let unavail = ManagerResult::Unavailable;
        assert_eq!(unavail.lookup_state("foo"), PackageState::Unavailable);
    }
}
