use std::path::Path;

use crate::error::{ForgeError, ForgeResult};
use crate::exec::CommandRunner;

// ---------------------------------------------------------------------------
// Read-only operations
// ---------------------------------------------------------------------------

/// Find files by name pattern (uses `find`)
pub fn find_file(runner: &dyn CommandRunner, pattern: &str) -> ForgeResult<String> {
    let output = runner.run("find", &[".", "-name", pattern])?;
    Ok(output.stdout)
}

/// Recursive content search (uses `grep -r`)
pub fn search(runner: &dyn CommandRunner, pattern: &str) -> ForgeResult<String> {
    let output = runner.run(
        "grep",
        &[
            "-rn",
            "--color=never",
            "--exclude-dir=.git",
            "--exclude-dir=.svn",
            "--exclude-dir=.idea",
            "--exclude-dir=node_modules",
            "--exclude-dir=target",
            pattern,
            ".",
        ],
    )?;
    Ok(output.stdout)
}

/// Spotlight search (macOS only, uses mdfind)
pub fn spotlight(runner: &dyn CommandRunner, query: &str) -> ForgeResult<String> {
    let output = runner.run("mdfind", &[query])?;
    Ok(output.stdout)
}

// ---------------------------------------------------------------------------
// Archive extraction
// ---------------------------------------------------------------------------

/// Supported archive format, determined by file extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveFormat {
    TarGz,
    TarBz2,
    TarXz,
    Tar,
    Zip,
    Gz,
    Bz2,
    Rar,
    SevenZip,
}

/// Detect archive format from file path extension.
pub fn detect_format(path: &str) -> Option<ArchiveFormat> {
    let lower = path.to_lowercase();
    if lower.ends_with(".tar.gz") || lower.ends_with(".tgz") {
        Some(ArchiveFormat::TarGz)
    } else if lower.ends_with(".tar.bz2") || lower.ends_with(".tbz2") {
        Some(ArchiveFormat::TarBz2)
    } else if lower.ends_with(".tar.xz") || lower.ends_with(".txz") {
        Some(ArchiveFormat::TarXz)
    } else if lower.ends_with(".tar") {
        Some(ArchiveFormat::Tar)
    } else if lower.ends_with(".zip") {
        Some(ArchiveFormat::Zip)
    } else if lower.ends_with(".gz") {
        Some(ArchiveFormat::Gz)
    } else if lower.ends_with(".bz2") {
        Some(ArchiveFormat::Bz2)
    } else if lower.ends_with(".rar") {
        Some(ArchiveFormat::Rar)
    } else if lower.ends_with(".7z") {
        Some(ArchiveFormat::SevenZip)
    } else {
        None
    }
}

/// Build the extraction command for a given format.
/// Returns (program, args).
pub fn extract_command(format: ArchiveFormat, file: &str) -> (&'static str, Vec<String>) {
    match format {
        ArchiveFormat::TarGz => ("tar", vec!["xzf".into(), file.into()]),
        ArchiveFormat::TarBz2 => ("tar", vec!["xjf".into(), file.into()]),
        ArchiveFormat::TarXz => ("tar", vec!["xJf".into(), file.into()]),
        ArchiveFormat::Tar => ("tar", vec!["xf".into(), file.into()]),
        ArchiveFormat::Zip => ("unzip", vec![file.into()]),
        ArchiveFormat::Gz => ("gunzip", vec![file.into()]),
        ArchiveFormat::Bz2 => ("bunzip2", vec![file.into()]),
        ArchiveFormat::Rar => ("unrar", vec!["x".into(), file.into()]),
        ArchiveFormat::SevenZip => ("7z", vec!["x".into(), file.into()]),
    }
}

/// Extract an archive file.
pub fn extract(runner: &dyn CommandRunner, file: &str) -> ForgeResult<String> {
    if !runner.is_dry_run() && !Path::new(file).exists() {
        return Err(ForgeError::CommandFailed(format!("File not found: {file}")));
    }

    let format = detect_format(file).ok_or_else(|| {
        ForgeError::CommandFailed(format!(
            "Unsupported archive format: {file}\n\
             Supported: .tar.gz, .tgz, .tar.bz2, .tbz2, .tar.xz, .txz, .tar, .zip, .gz, .bz2, .rar, .7z"
        ))
    })?;

    let (program, args) = extract_command(format, file);
    let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let output = runner.run(program, &args_refs)?;

    if runner.is_dry_run() {
        return Ok(String::new());
    }

    if output.success() {
        Ok(format!("Extracted: {file}"))
    } else {
        let msg = output.stderr.lines().next().unwrap_or("extraction failed");
        Err(ForgeError::CommandFailed(format!(
            "Failed to extract {file}: {msg}"
        )))
    }
}

// ---------------------------------------------------------------------------
// Trash (safe delete)
// ---------------------------------------------------------------------------

/// Move a file or directory to the system trash.
///
/// On macOS, uses `trash` (from Homebrew) if available, falling back to
/// moving to `~/.Trash/`.
/// On Linux, uses `trash-put` (from trash-cli) if available, falling back
/// to moving to `~/.local/share/Trash/files/`.
pub fn trash(runner: &dyn CommandRunner, file: &str) -> ForgeResult<String> {
    if !runner.is_dry_run() && !Path::new(file).exists() {
        return Err(ForgeError::CommandFailed(format!("File not found: {file}")));
    }

    // Try platform-appropriate trash commands
    if try_trash_command(runner, file, "trash", &[file])? {
        return Ok(format!("Trashed: {file}"));
    }
    if try_trash_command(runner, file, "trash-put", &[file])? {
        return Ok(format!("Trashed: {file}"));
    }

    // Fallback: move to trash directory
    if runner.is_dry_run() {
        return Ok(String::new());
    }

    let trash_dir = resolve_trash_dir();
    std::fs::create_dir_all(&trash_dir)
        .map_err(|e| ForgeError::CommandFailed(format!("Failed to create trash directory: {e}")))?;

    let filename = Path::new(file)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let dest = Path::new(&trash_dir).join(&filename);
    let output = runner.run("mv", &[file, &dest.to_string_lossy()])?;

    if output.success() {
        Ok(format!("Moved to trash: {file} → {}", dest.display()))
    } else {
        Err(ForgeError::CommandFailed(format!(
            "Failed to trash {file}: {}",
            output.stderr.lines().next().unwrap_or("mv failed")
        )))
    }
}

/// Try a specific trash command; return Ok(true) if it worked.
fn try_trash_command(
    runner: &dyn CommandRunner,
    _file: &str,
    program: &str,
    args: &[&str],
) -> ForgeResult<bool> {
    // Check if the command exists
    let which = runner.run("which", &[program]);
    match which {
        Ok(output) if output.success() => {}
        _ => return Ok(false),
    }

    let output = runner.run(program, args)?;
    if runner.is_dry_run() || output.success() {
        Ok(true)
    } else {
        Ok(false)
    }
}

fn resolve_trash_dir() -> String {
    if cfg!(target_os = "macos") {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        format!("{home}/.Trash")
    } else {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        format!("{home}/.local/share/Trash/files")
    }
}

// ---------------------------------------------------------------------------
// DS_Store cleanup
// ---------------------------------------------------------------------------

/// Find all .DS_Store files recursively from the current directory.
/// Returns the list of paths found.
pub fn find_ds_store(runner: &dyn CommandRunner) -> ForgeResult<Vec<String>> {
    let output = runner.run("find", &[".", "-name", ".DS_Store", "-type", "f"])?;

    if runner.is_dry_run() {
        return Ok(Vec::new());
    }

    let files: Vec<String> = output
        .stdout
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.to_string())
        .collect();

    Ok(files)
}

/// Remove the given .DS_Store files.
pub fn remove_ds_store(runner: &dyn CommandRunner, files: &[String]) -> ForgeResult<String> {
    if files.is_empty() {
        return Ok("No .DS_Store files to remove.".to_string());
    }

    let mut removed = 0;
    let mut errors = Vec::new();

    for file in files {
        let output = runner.run("rm", &[file.as_str()])?;
        if runner.is_dry_run() || output.success() {
            removed += 1;
        } else {
            errors.push(file.clone());
        }
    }

    if errors.is_empty() {
        Ok(format!("Removed {removed} .DS_Store file(s)."))
    } else {
        Ok(format!(
            "Removed {removed} file(s). Failed to remove: {}",
            errors.join(", ")
        ))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_format_tar_gz() {
        assert_eq!(detect_format("foo.tar.gz"), Some(ArchiveFormat::TarGz));
        assert_eq!(detect_format("foo.tgz"), Some(ArchiveFormat::TarGz));
        assert_eq!(detect_format("FOO.TAR.GZ"), Some(ArchiveFormat::TarGz));
    }

    #[test]
    fn test_detect_format_tar_bz2() {
        assert_eq!(detect_format("foo.tar.bz2"), Some(ArchiveFormat::TarBz2));
        assert_eq!(detect_format("foo.tbz2"), Some(ArchiveFormat::TarBz2));
    }

    #[test]
    fn test_detect_format_tar_xz() {
        assert_eq!(detect_format("foo.tar.xz"), Some(ArchiveFormat::TarXz));
        assert_eq!(detect_format("foo.txz"), Some(ArchiveFormat::TarXz));
    }

    #[test]
    fn test_detect_format_simple() {
        assert_eq!(detect_format("foo.tar"), Some(ArchiveFormat::Tar));
        assert_eq!(detect_format("foo.zip"), Some(ArchiveFormat::Zip));
        assert_eq!(detect_format("foo.gz"), Some(ArchiveFormat::Gz));
        assert_eq!(detect_format("foo.bz2"), Some(ArchiveFormat::Bz2));
        assert_eq!(detect_format("foo.rar"), Some(ArchiveFormat::Rar));
        assert_eq!(detect_format("foo.7z"), Some(ArchiveFormat::SevenZip));
    }

    #[test]
    fn test_detect_format_unsupported() {
        assert_eq!(detect_format("foo.txt"), None);
        assert_eq!(detect_format("foo.pdf"), None);
        assert_eq!(detect_format("foo"), None);
    }

    #[test]
    fn test_extract_command_tar_gz() {
        let (prog, args) = extract_command(ArchiveFormat::TarGz, "test.tar.gz");
        assert_eq!(prog, "tar");
        assert_eq!(args, vec!["xzf", "test.tar.gz"]);
    }

    #[test]
    fn test_extract_command_zip() {
        let (prog, args) = extract_command(ArchiveFormat::Zip, "test.zip");
        assert_eq!(prog, "unzip");
        assert_eq!(args, vec!["test.zip"]);
    }

    #[test]
    fn test_extract_command_7z() {
        let (prog, args) = extract_command(ArchiveFormat::SevenZip, "test.7z");
        assert_eq!(prog, "7z");
        assert_eq!(args, vec!["x", "test.7z"]);
    }

    #[test]
    fn test_extract_dry_run() {
        use crate::exec::DryRunRunner;
        let runner = DryRunRunner;
        let result = extract(&runner, "test.tar.gz").unwrap();
        assert!(result.is_empty()); // dry-run produces no output
    }

    #[test]
    fn test_extract_unsupported_format() {
        use crate::exec::DryRunRunner;
        let runner = DryRunRunner;
        let result = extract(&runner, "test.txt");
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("Unsupported archive format"));
    }

    #[test]
    fn test_find_ds_store_dry_run() {
        use crate::exec::DryRunRunner;
        let runner = DryRunRunner;
        let files = find_ds_store(&runner).unwrap();
        assert!(files.is_empty());
    }

    #[test]
    fn test_remove_ds_store_empty() {
        use crate::exec::DryRunRunner;
        let runner = DryRunRunner;
        let result = remove_ds_store(&runner, &[]).unwrap();
        assert_eq!(result, "No .DS_Store files to remove.");
    }

    #[test]
    fn test_remove_ds_store_dry_run() {
        use crate::exec::DryRunRunner;
        let runner = DryRunRunner;
        let files = vec!["./.DS_Store".to_string(), "./src/.DS_Store".to_string()];
        let result = remove_ds_store(&runner, &files).unwrap();
        assert!(result.contains("Removed 2"));
    }

    #[test]
    fn test_trash_dry_run() {
        use crate::exec::DryRunRunner;
        let runner = DryRunRunner;
        // In dry-run, which + trash commands are printed but not executed.
        // DryRunRunner returns success for `which`, so try_trash_command
        // proceeds to run the trash command (also dry-run) and reports success.
        let result = trash(&runner, "somefile.txt").unwrap();
        assert!(result.contains("Trashed") || result.is_empty());
    }
}
