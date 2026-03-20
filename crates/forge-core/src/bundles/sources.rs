//! Detection adapters for package managers, version managers, and standalone binaries.
//!
//! Every adapter follows the same pattern:
//!   1. Check if the tool exists (best-effort, via `which`)
//!   2. Run a list/status command and parse stdout
//!   3. Return a `Vec<DetectedPackage>` (may be empty)
//!
//! All functions are read-only and never mutate anything.

use std::path::{Path, PathBuf};

use crate::error::ForgeResult;
use crate::exec::CommandRunner;

// ---------------------------------------------------------------------------
// Source classification
// ---------------------------------------------------------------------------

/// How a tool was installed / where it was found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallSource {
    /// Installed via a package manager (brew, npm, pipx, …)
    PackageManager(PackageManagerKind),
    /// Managed by a version/toolchain manager (asdf, mise)
    VersionManager(VersionManagerKind),
    /// Binary found in a well-known directory
    StandaloneBinary { dir: PathBuf },
    /// Matched via a manual `check_command`
    Manual,
}

impl std::fmt::Display for InstallSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PackageManager(kind) => write!(f, "{kind}"),
            Self::VersionManager(kind) => write!(f, "{kind}"),
            Self::StandaloneBinary { dir } => write!(f, "binary ({})", dir.display()),
            Self::Manual => write!(f, "manual"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageManagerKind {
    Brew,
    BrewCask,
    Npm,
    Pnpm,
    Bun,
    Go,
    Gem,
    Pipx,
    UvTool,
    Composer,
}

impl std::fmt::Display for PackageManagerKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Brew => write!(f, "brew"),
            Self::BrewCask => write!(f, "brew-cask"),
            Self::Npm => write!(f, "npm"),
            Self::Pnpm => write!(f, "pnpm"),
            Self::Bun => write!(f, "bun"),
            Self::Go => write!(f, "go"),
            Self::Gem => write!(f, "gem"),
            Self::Pipx => write!(f, "pipx"),
            Self::UvTool => write!(f, "uv-tool"),
            Self::Composer => write!(f, "composer"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionManagerKind {
    Asdf,
    Mise,
}

impl std::fmt::Display for VersionManagerKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Asdf => write!(f, "asdf"),
            Self::Mise => write!(f, "mise"),
        }
    }
}

/// The category a source belongs to — used for display grouping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceCategory {
    PackageManager,
    VersionManager,
    StandaloneBinary,
    Manual,
}

impl InstallSource {
    pub fn category(&self) -> SourceCategory {
        match self {
            Self::PackageManager(_) => SourceCategory::PackageManager,
            Self::VersionManager(_) => SourceCategory::VersionManager,
            Self::StandaloneBinary { .. } => SourceCategory::StandaloneBinary,
            Self::Manual => SourceCategory::Manual,
        }
    }
}

// ---------------------------------------------------------------------------
// Detected package — richer than the old PackageStatus
// ---------------------------------------------------------------------------

/// A package/tool detected on the system.
#[derive(Debug, Clone)]
pub struct DetectedPackage {
    pub name: String,
    /// Version string if the adapter was able to extract it.
    pub version: Option<String>,
    pub source: InstallSource,
}

// ---------------------------------------------------------------------------
// Adapter availability check (helper)
// ---------------------------------------------------------------------------

fn tool_exists(runner: &dyn CommandRunner, cmd: &str) -> bool {
    if runner.is_dry_run() {
        return false;
    }
    runner
        .run("which", &[cmd])
        .map(|o| o.success())
        .unwrap_or(false)
}

/// Run a command and return its stdout, or `None` if it fails.
fn run_stdout(runner: &dyn CommandRunner, cmd: &str, args: &[&str]) -> Option<String> {
    if runner.is_dry_run() {
        return None;
    }
    runner
        .run(cmd, args)
        .ok()
        .and_then(|o| if o.success() { Some(o.stdout) } else { None })
}

// ---------------------------------------------------------------------------
// Package-manager adapters
// ---------------------------------------------------------------------------

/// List globally-installed pipx packages.
///
/// `pipx list --short` outputs lines like:
///   package-name 1.2.3
pub fn detect_pipx(runner: &dyn CommandRunner) -> ForgeResult<Vec<DetectedPackage>> {
    if !tool_exists(runner, "pipx") {
        return Ok(Vec::new());
    }
    let stdout = match run_stdout(runner, "pipx", &["list", "--short"]) {
        Some(s) => s,
        None => return Ok(Vec::new()),
    };
    Ok(parse_pipx_output(&stdout))
}

pub fn parse_pipx_output(stdout: &str) -> Vec<DetectedPackage> {
    stdout
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let name = parts.next()?;
            let version = parts.next().map(|v| v.to_string());
            Some(DetectedPackage {
                name: name.to_string(),
                version,
                source: InstallSource::PackageManager(PackageManagerKind::Pipx),
            })
        })
        .collect()
}

/// List tools installed via `uv tool`.
///
/// `uv tool list` outputs lines like:
///   ruff v0.4.1
///   - ruff
///
/// We want the top-level lines (the ones without leading whitespace/dash).
pub fn detect_uv_tool(runner: &dyn CommandRunner) -> ForgeResult<Vec<DetectedPackage>> {
    if !tool_exists(runner, "uv") {
        return Ok(Vec::new());
    }
    let stdout = match run_stdout(runner, "uv", &["tool", "list"]) {
        Some(s) => s,
        None => return Ok(Vec::new()),
    };
    Ok(parse_uv_tool_output(&stdout))
}

pub fn parse_uv_tool_output(stdout: &str) -> Vec<DetectedPackage> {
    stdout
        .lines()
        .filter(|l| !l.starts_with(' ') && !l.starts_with('-') && !l.trim().is_empty())
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let name = parts.next()?;
            // Skip noise lines like "No tools installed", "Warning:", etc.
            if name
                .chars()
                .next()
                .map(|c| c.is_uppercase())
                .unwrap_or(true)
            {
                return None;
            }
            let version = parts.next().map(|v| v.trim_start_matches('v').to_string());
            Some(DetectedPackage {
                name: name.to_string(),
                version,
                source: InstallSource::PackageManager(PackageManagerKind::UvTool),
            })
        })
        .collect()
}

/// List globally-installed pnpm packages.
///
/// `pnpm list -g --depth=0` produces a table; `--parseable` gives paths.
/// We use the parseable mode for robustness.
pub fn detect_pnpm(runner: &dyn CommandRunner) -> ForgeResult<Vec<DetectedPackage>> {
    if !tool_exists(runner, "pnpm") {
        return Ok(Vec::new());
    }
    let stdout = match run_stdout(runner, "pnpm", &["list", "-g", "--depth=0", "--parseable"]) {
        Some(s) => s,
        None => return Ok(Vec::new()),
    };
    Ok(parse_pnpm_output(&stdout))
}

pub fn parse_pnpm_output(stdout: &str) -> Vec<DetectedPackage> {
    stdout
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|line| {
            // Lines look like:
            //   /path/to/global/node_modules/typescript
            //   /path/to/global/node_modules/@openai/codex
            // We split at "node_modules/" and take everything after.
            if !line.contains("node_modules/") {
                return None;
            }
            let after_nm = line.rsplit("node_modules/").next()?;
            if after_nm.is_empty() {
                return None;
            }
            Some(DetectedPackage {
                name: after_nm.to_string(),
                version: None,
                source: InstallSource::PackageManager(PackageManagerKind::Pnpm),
            })
        })
        .collect()
}

/// List globally-installed bun packages.
///
/// `bun pm ls -g` outputs a tree; we parse root-level entries.
/// Lines look like:
///   /path/to/global node_modules
///   ├── package@version
///   └── package@version
pub fn detect_bun(runner: &dyn CommandRunner) -> ForgeResult<Vec<DetectedPackage>> {
    if !tool_exists(runner, "bun") {
        return Ok(Vec::new());
    }
    let stdout = match run_stdout(runner, "bun", &["pm", "ls", "-g"]) {
        Some(s) => s,
        None => return Ok(Vec::new()),
    };
    Ok(parse_bun_output(&stdout))
}

pub fn parse_bun_output(stdout: &str) -> Vec<DetectedPackage> {
    stdout
        .lines()
        .filter_map(|line| {
            // Skip header lines, look for tree entries
            let trimmed = line.trim_start_matches(|c: char| "├─└│ \t".contains(c));
            if trimmed.is_empty() || trimmed.starts_with('/') {
                return None;
            }
            // Format: package@version or @scope/package@version
            let (name, version) = if let Some(at_pos) = trimmed.rfind('@') {
                if at_pos == 0 {
                    // Scoped package: @scope/name@version
                    // Find the second '@'
                    if let Some(pos) = trimmed[1..].find('@') {
                        let split = pos + 1;
                        (
                            trimmed[..split].to_string(),
                            Some(trimmed[split + 1..].to_string()),
                        )
                    } else {
                        (trimmed.to_string(), None)
                    }
                } else {
                    (
                        trimmed[..at_pos].to_string(),
                        Some(trimmed[at_pos + 1..].to_string()),
                    )
                }
            } else {
                (trimmed.to_string(), None)
            };
            Some(DetectedPackage {
                name,
                version,
                source: InstallSource::PackageManager(PackageManagerKind::Bun),
            })
        })
        .collect()
}

/// List globally-installed Composer packages.
///
/// `composer global show --name-only` outputs one package per line:
///   vendor/package
pub fn detect_composer(runner: &dyn CommandRunner) -> ForgeResult<Vec<DetectedPackage>> {
    if !tool_exists(runner, "composer") {
        return Ok(Vec::new());
    }
    let stdout = match run_stdout(runner, "composer", &["global", "show", "--name-only"]) {
        Some(s) => s,
        None => return Ok(Vec::new()),
    };
    Ok(parse_composer_output(&stdout))
}

pub fn parse_composer_output(stdout: &str) -> Vec<DetectedPackage> {
    stdout
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with("Changed") && !l.starts_with("No "))
        .map(|line| DetectedPackage {
            name: line.trim().to_string(),
            version: None,
            source: InstallSource::PackageManager(PackageManagerKind::Composer),
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Version-manager adapters
// ---------------------------------------------------------------------------

/// List asdf plugins and their installed versions.
///
/// `asdf list` outputs:
///   golang
///     1.21.5
///     1.22.0
///   nodejs
///     20.11.0
///
/// We report one entry per plugin (not per version).
pub fn detect_asdf(runner: &dyn CommandRunner) -> ForgeResult<Vec<DetectedPackage>> {
    if !tool_exists(runner, "asdf") {
        return Ok(Vec::new());
    }
    let stdout = match run_stdout(runner, "asdf", &["list"]) {
        Some(s) => s,
        None => return Ok(Vec::new()),
    };
    Ok(parse_asdf_output(&stdout))
}

pub fn parse_asdf_output(stdout: &str) -> Vec<DetectedPackage> {
    let mut results = Vec::new();
    let mut current_plugin: Option<String> = None;
    let mut current_versions: Vec<String> = Vec::new();

    for line in stdout.lines() {
        if line.is_empty() {
            continue;
        }
        if !line.starts_with(' ') && !line.starts_with('\t') {
            // Flush previous plugin
            if let Some(plugin) = current_plugin.take() {
                let version = current_versions.last().cloned();
                results.push(DetectedPackage {
                    name: plugin,
                    version,
                    source: InstallSource::VersionManager(VersionManagerKind::Asdf),
                });
                current_versions.clear();
            }
            current_plugin = Some(line.trim().to_string());
        } else {
            let v = line.trim().trim_start_matches('*').trim().to_string();
            if !v.is_empty() && !v.starts_with("No ") {
                current_versions.push(v);
            }
        }
    }
    // Flush last plugin
    if let Some(plugin) = current_plugin {
        let version = current_versions.last().cloned();
        results.push(DetectedPackage {
            name: plugin,
            version,
            source: InstallSource::VersionManager(VersionManagerKind::Asdf),
        });
    }

    results
}

/// List mise toolchains.
///
/// `mise list --current` outputs a table like:
///   Tool    Version  Config Source           Requested
///   go      1.22.0   ~/.tool-versions       1.22.0
///   node    20.11.0  ~/.tool-versions       20
///
/// `mise list` (without --current) may differ. We try `--current` first.
pub fn detect_mise(runner: &dyn CommandRunner) -> ForgeResult<Vec<DetectedPackage>> {
    if !tool_exists(runner, "mise") {
        return Ok(Vec::new());
    }
    // Try --current first (shows active toolchains)
    let stdout = run_stdout(runner, "mise", &["list", "--current"])
        .or_else(|| run_stdout(runner, "mise", &["list"]));
    match stdout {
        Some(s) => Ok(parse_mise_output(&s)),
        None => Ok(Vec::new()),
    }
}

pub fn parse_mise_output(stdout: &str) -> Vec<DetectedPackage> {
    stdout
        .lines()
        .skip(1) // skip header row
        .filter(|l| !l.trim().is_empty())
        .filter_map(|line| {
            let mut cols = line.split_whitespace();
            let name = cols.next()?;
            // Skip header guard
            if name == "Tool" || name == "---" || name.starts_with('-') {
                return None;
            }
            let version = cols.next().map(|v| v.to_string());
            Some(DetectedPackage {
                name: name.to_string(),
                version,
                source: InstallSource::VersionManager(VersionManagerKind::Mise),
            })
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Standalone binary scan
// ---------------------------------------------------------------------------

/// Default bin directories to scan.
pub fn default_bin_dirs() -> Vec<PathBuf> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let mut dirs = vec![
        PathBuf::from(format!("{home}/.local/bin")),
        PathBuf::from(format!("{home}/bin")),
        PathBuf::from("/usr/local/bin"),
        PathBuf::from("/opt/homebrew/bin"),
        PathBuf::from(format!("{home}/.cargo/bin")),
    ];

    // Add $(go env GOPATH)/bin if available
    if let Ok(gopath) = std::env::var("GOPATH") {
        dirs.push(PathBuf::from(format!("{gopath}/bin")));
    } else {
        // Default GOPATH is ~/go
        dirs.push(PathBuf::from(format!("{home}/go/bin")));
    }

    dirs
}

/// Scan a directory for executable binaries.
/// Returns one `DetectedPackage` per executable file found.
pub fn scan_bin_dir(dir: &Path) -> Vec<DetectedPackage> {
    if !dir.is_dir() {
        return Vec::new();
    }

    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    entries
        .filter_map(|e| e.ok())
        .filter_map(|entry| {
            let path = entry.path();
            if !path.is_file() {
                return None;
            }
            // Check executable bit (Unix)
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let meta = path.metadata().ok()?;
                if meta.permissions().mode() & 0o111 == 0 {
                    return None;
                }
            }
            let name = path.file_name()?.to_string_lossy().to_string();
            // Skip hidden files and common non-tool entries
            if name.starts_with('.') {
                return None;
            }
            Some(DetectedPackage {
                name,
                version: None,
                source: InstallSource::StandaloneBinary {
                    dir: dir.to_path_buf(),
                },
            })
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Environment scan — aggregated view of everything on the machine
// ---------------------------------------------------------------------------

/// Result of scanning the full environment for installed tools.
#[derive(Debug)]
pub struct EnvironmentScan {
    /// Tools detected via package managers.
    pub package_manager: Vec<DetectedPackage>,
    /// Tools detected via version managers.
    pub version_manager: Vec<DetectedPackage>,
    /// Executables found in well-known bin directories.
    pub standalone_binaries: Vec<(PathBuf, Vec<String>)>,
    /// Managers that were checked but not found on PATH.
    pub unavailable_managers: Vec<String>,
}

impl EnvironmentScan {
    /// Run all detectors. Best-effort: individual failures are swallowed.
    pub fn run(runner: &dyn CommandRunner) -> Self {
        let mut package_manager = Vec::new();
        let mut version_manager = Vec::new();
        let mut unavailable_managers = Vec::new();

        type DetectFn = fn(&dyn CommandRunner) -> ForgeResult<Vec<DetectedPackage>>;

        // -- Package managers --
        let pm_adapters: Vec<(&str, DetectFn)> = vec![
            ("pipx", detect_pipx),
            ("uv tool", detect_uv_tool),
            ("pnpm", detect_pnpm),
            ("bun", detect_bun),
            ("composer", detect_composer),
        ];

        for (label, detect_fn) in &pm_adapters {
            match detect_fn(runner) {
                Ok(pkgs) if pkgs.is_empty() => {
                    // The adapter ran but found nothing — could mean:
                    // a) tool exists with no global packages, or
                    // b) tool not installed.
                    // We distinguish by checking if the tool binary exists.
                    let cmd = label.split_whitespace().next().unwrap_or(label);
                    if !tool_exists(runner, cmd) {
                        unavailable_managers.push(label.to_string());
                    }
                }
                Ok(pkgs) => package_manager.extend(pkgs),
                Err(_) => {
                    unavailable_managers.push(label.to_string());
                }
            }
        }

        // -- Version managers --
        let vm_adapters: Vec<(&str, DetectFn)> = vec![("asdf", detect_asdf), ("mise", detect_mise)];

        for (label, detect_fn) in &vm_adapters {
            match detect_fn(runner) {
                Ok(pkgs) if pkgs.is_empty() => {
                    if !tool_exists(runner, label) {
                        unavailable_managers.push(label.to_string());
                    }
                }
                Ok(pkgs) => version_manager.extend(pkgs),
                Err(_) => {
                    unavailable_managers.push(label.to_string());
                }
            }
        }

        // -- Bin directories --
        let bin_dirs = default_bin_dirs();
        let standalone_binaries: Vec<(PathBuf, Vec<String>)> = bin_dirs
            .into_iter()
            .filter_map(|dir| {
                let binaries = scan_bin_dir(&dir);
                if binaries.is_empty() {
                    return None;
                }
                let names: Vec<String> = binaries.into_iter().map(|p| p.name).collect();
                Some((dir, names))
            })
            .collect();

        Self {
            package_manager,
            version_manager,
            standalone_binaries,
            unavailable_managers,
        }
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- pipx parser --

    #[test]
    fn test_parse_pipx_basic() {
        let input = "\
black 24.3.0
ruff 0.4.1
poetry 1.8.2
";
        let pkgs = parse_pipx_output(input);
        assert_eq!(pkgs.len(), 3);
        assert_eq!(pkgs[0].name, "black");
        assert_eq!(pkgs[0].version.as_deref(), Some("24.3.0"));
        assert_eq!(pkgs[1].name, "ruff");
        assert_eq!(
            pkgs[0].source,
            InstallSource::PackageManager(PackageManagerKind::Pipx)
        );
    }

    #[test]
    fn test_parse_pipx_empty() {
        assert!(parse_pipx_output("").is_empty());
        assert!(parse_pipx_output("   \n  \n").is_empty());
    }

    // -- uv tool parser --

    #[test]
    fn test_parse_uv_tool_basic() {
        let input = "\
ruff v0.4.1
- ruff
black v24.3.0
- black
- blackd
";
        let pkgs = parse_uv_tool_output(input);
        assert_eq!(pkgs.len(), 2);
        assert_eq!(pkgs[0].name, "ruff");
        assert_eq!(pkgs[0].version.as_deref(), Some("0.4.1")); // v stripped
        assert_eq!(pkgs[1].name, "black");
        assert_eq!(
            pkgs[0].source,
            InstallSource::PackageManager(PackageManagerKind::UvTool)
        );
    }

    #[test]
    fn test_parse_uv_tool_empty() {
        assert!(parse_uv_tool_output("").is_empty());
        assert!(parse_uv_tool_output("No tools installed").is_empty());
    }

    // -- pnpm parser --

    #[test]
    fn test_parse_pnpm_basic() {
        let input = "\
/Users/me/Library/pnpm/global/5
/Users/me/Library/pnpm/global/5/node_modules/typescript
/Users/me/Library/pnpm/global/5/node_modules/eslint
";
        let pkgs = parse_pnpm_output(input);
        assert_eq!(pkgs.len(), 2);
        assert_eq!(pkgs[0].name, "typescript");
        assert_eq!(pkgs[1].name, "eslint");
        assert_eq!(
            pkgs[0].source,
            InstallSource::PackageManager(PackageManagerKind::Pnpm)
        );
    }

    #[test]
    fn test_parse_pnpm_scoped() {
        let input = "\
/Users/me/Library/pnpm/global/5
/Users/me/Library/pnpm/global/5/node_modules/@openai/codex
/Users/me/Library/pnpm/global/5/node_modules/@google/gemini-cli
/Users/me/Library/pnpm/global/5/node_modules/typescript
";
        let pkgs = parse_pnpm_output(input);
        assert_eq!(pkgs.len(), 3);
        assert_eq!(pkgs[0].name, "@openai/codex");
        assert_eq!(pkgs[1].name, "@google/gemini-cli");
        assert_eq!(pkgs[2].name, "typescript");
    }

    // -- bun parser --

    #[test]
    fn test_parse_bun_tree() {
        let input = "\
/Users/me/.bun/install/global node_modules
├── typescript@5.4.3
└── @biomejs/biome@1.7.0
";
        let pkgs = parse_bun_output(input);
        assert_eq!(pkgs.len(), 2);
        assert_eq!(pkgs[0].name, "typescript");
        assert_eq!(pkgs[0].version.as_deref(), Some("5.4.3"));
        assert_eq!(pkgs[1].name, "@biomejs/biome");
        assert_eq!(pkgs[1].version.as_deref(), Some("1.7.0"));
    }

    #[test]
    fn test_parse_bun_empty() {
        assert!(parse_bun_output("").is_empty());
    }

    // -- composer parser --

    #[test]
    fn test_parse_composer_basic() {
        let input = "\
laravel/installer
squizlabs/php_codesniffer
phpunit/phpunit
";
        let pkgs = parse_composer_output(input);
        assert_eq!(pkgs.len(), 3);
        assert_eq!(pkgs[0].name, "laravel/installer");
        assert_eq!(
            pkgs[0].source,
            InstallSource::PackageManager(PackageManagerKind::Composer)
        );
    }

    #[test]
    fn test_parse_composer_with_noise() {
        let input = "Changed current directory to /Users/me/.composer\nlaravel/installer\n";
        let pkgs = parse_composer_output(input);
        assert_eq!(pkgs.len(), 1);
        assert_eq!(pkgs[0].name, "laravel/installer");
    }

    #[test]
    fn test_parse_composer_empty() {
        let input = "No packages installed\n";
        let pkgs = parse_composer_output(input);
        assert!(pkgs.is_empty());
    }

    // -- asdf parser --

    #[test]
    fn test_parse_asdf_basic() {
        let input = "\
golang
  1.21.5
  1.22.0
nodejs
  20.11.0
";
        let pkgs = parse_asdf_output(input);
        assert_eq!(pkgs.len(), 2);
        assert_eq!(pkgs[0].name, "golang");
        assert_eq!(pkgs[0].version.as_deref(), Some("1.22.0")); // latest
        assert_eq!(pkgs[1].name, "nodejs");
        assert_eq!(pkgs[1].version.as_deref(), Some("20.11.0"));
        assert_eq!(
            pkgs[0].source,
            InstallSource::VersionManager(VersionManagerKind::Asdf)
        );
    }

    #[test]
    fn test_parse_asdf_with_star() {
        let input = "\
python
 *3.12.1
  3.11.0
";
        let pkgs = parse_asdf_output(input);
        assert_eq!(pkgs.len(), 1);
        assert_eq!(pkgs[0].name, "python");
        // Last version wins
        assert_eq!(pkgs[0].version.as_deref(), Some("3.11.0"));
    }

    #[test]
    fn test_parse_asdf_no_versions() {
        let input = "\
golang
  No versions installed
";
        let pkgs = parse_asdf_output(input);
        assert_eq!(pkgs.len(), 1);
        assert_eq!(pkgs[0].name, "golang");
        assert_eq!(pkgs[0].version, None);
    }

    #[test]
    fn test_parse_asdf_empty() {
        assert!(parse_asdf_output("").is_empty());
    }

    // -- mise parser --

    #[test]
    fn test_parse_mise_basic() {
        let input = "\
Tool    Version  Config Source           Requested
go      1.22.0   ~/.tool-versions       1.22.0
node    20.11.0  ~/.tool-versions       20
python  3.12.1   ~/project/.tool-versions 3.12
";
        let pkgs = parse_mise_output(input);
        assert_eq!(pkgs.len(), 3);
        assert_eq!(pkgs[0].name, "go");
        assert_eq!(pkgs[0].version.as_deref(), Some("1.22.0"));
        assert_eq!(pkgs[1].name, "node");
        assert_eq!(pkgs[2].name, "python");
        assert_eq!(
            pkgs[0].source,
            InstallSource::VersionManager(VersionManagerKind::Mise)
        );
    }

    #[test]
    fn test_parse_mise_empty() {
        let input = "Tool    Version  Config Source           Requested\n";
        let pkgs = parse_mise_output(input);
        assert!(pkgs.is_empty());
    }

    // -- source display --

    #[test]
    fn test_source_display() {
        assert_eq!(
            InstallSource::PackageManager(PackageManagerKind::Pipx).to_string(),
            "pipx"
        );
        assert_eq!(
            InstallSource::VersionManager(VersionManagerKind::Mise).to_string(),
            "mise"
        );
        assert_eq!(
            InstallSource::StandaloneBinary {
                dir: PathBuf::from("/usr/local/bin")
            }
            .to_string(),
            "binary (/usr/local/bin)"
        );
    }
}
