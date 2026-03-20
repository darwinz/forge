use crate::error::ForgeResult;
use crate::exec::CommandRunner;

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
