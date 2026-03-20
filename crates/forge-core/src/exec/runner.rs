use crate::error::ForgeResult;

/// Output from a command execution
#[derive(Debug, Clone)]
pub struct CommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub status_code: i32,
}

impl CommandOutput {
    pub fn success(&self) -> bool {
        self.status_code == 0
    }
}

/// Trait for executing external commands
pub trait CommandRunner {
    /// Run a command and capture output
    fn run(&self, program: &str, args: &[&str]) -> ForgeResult<CommandOutput>;

    /// Run a command with piped output (for display to user)
    fn run_piped(&self, program: &str, args: &[&str]) -> ForgeResult<CommandOutput>;

    /// Check if dry-run mode is active
    fn is_dry_run(&self) -> bool;
}

/// Real command runner that actually executes commands
pub struct RealRunner;

impl CommandRunner for RealRunner {
    fn run(&self, program: &str, args: &[&str]) -> ForgeResult<CommandOutput> {
        tracing::debug!(program, ?args, "executing command");

        let output = std::process::Command::new(program)
            .args(args)
            .output()
            .map_err(|e| crate::ForgeError::CommandFailed(
                format!("failed to execute {program}: {e}")
            ))?;

        Ok(CommandOutput {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            status_code: output.status.code().unwrap_or(-1),
        })
    }

    fn run_piped(&self, program: &str, args: &[&str]) -> ForgeResult<CommandOutput> {
        self.run(program, args)
    }

    fn is_dry_run(&self) -> bool {
        false
    }
}
