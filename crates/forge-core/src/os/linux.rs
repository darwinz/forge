use crate::exec::CommandRunner;
use crate::error::ForgeResult;
use super::OsPlatform;

pub struct Linux;

impl OsPlatform for Linux {
    fn hardware_info(&self, runner: &dyn CommandRunner) -> ForgeResult<String> {
        let output = runner.run("lshw", &[])?;
        Ok(output.stdout)
    }

    fn ip_info(&self, runner: &dyn CommandRunner, interface: &str) -> ForgeResult<String> {
        let output = runner.run("ip", &["addr", "show", interface])?;
        Ok(output.stdout)
    }

    fn platform_name(&self) -> &str {
        "Linux"
    }
}
