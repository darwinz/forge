use crate::error::ForgeResult;
use super::runner::{CommandRunner, CommandOutput};

/// Dry-run command runner that prints commands without executing
pub struct DryRunRunner;

impl CommandRunner for DryRunRunner {
    fn run(&self, program: &str, args: &[&str]) -> ForgeResult<CommandOutput> {
        let cmd_str = format!("{} {}", program, args.join(" "));
        tracing::info!("[dry-run] would execute: {}", cmd_str);
        println!("[dry-run] would execute: {cmd_str}");

        Ok(CommandOutput {
            stdout: String::new(),
            stderr: String::new(),
            status_code: 0,
        })
    }

    fn run_piped(&self, program: &str, args: &[&str]) -> ForgeResult<CommandOutput> {
        self.run(program, args)
    }

    fn is_dry_run(&self) -> bool {
        true
    }
}
