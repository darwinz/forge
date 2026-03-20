use crate::exec::CommandRunner;
use crate::error::ForgeResult;
use super::OsPlatform;

pub struct MacOs;

impl OsPlatform for MacOs {
    fn hardware_info(&self, runner: &dyn CommandRunner) -> ForgeResult<String> {
        let output = runner.run("system_profiler", &["SPHardwareDataType"])?;
        if runner.is_dry_run() {
            runner.run("diskutil", &["list"])?;
            return Ok(String::new());
        }

        let mut result = output.stdout;

        // Also get disk info
        if let Ok(disk_output) = runner.run("diskutil", &["list"]) {
            result.push('\n');
            result.push_str(&disk_output.stdout);
        }

        Ok(result)
    }

    fn ip_info(&self, runner: &dyn CommandRunner, interface: &str) -> ForgeResult<String> {
        let output = runner.run("ipconfig", &["getpacket", interface])?;
        Ok(output.stdout)
    }

    fn platform_name(&self) -> &str {
        "macOS"
    }
}
