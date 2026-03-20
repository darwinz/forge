pub mod macos;
pub mod linux;

use crate::exec::CommandRunner;
use crate::error::ForgeResult;

/// Platform abstraction for OS-specific operations.
///
/// All methods take a `CommandRunner` to support dry-run mode.
pub trait OsPlatform {
    /// Get system hardware info
    fn hardware_info(&self, runner: &dyn CommandRunner) -> ForgeResult<String>;

    /// Get network interface info
    fn ip_info(&self, runner: &dyn CommandRunner, interface: &str) -> ForgeResult<String>;

    /// Get the current platform name
    fn platform_name(&self) -> &str;
}

/// Detect and return the current platform
pub fn current_platform() -> Box<dyn OsPlatform> {
    #[cfg(target_os = "macos")]
    {
        Box::new(macos::MacOs)
    }
    #[cfg(target_os = "linux")]
    {
        Box::new(linux::Linux)
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        Box::new(linux::Linux) // fallback
    }
}
