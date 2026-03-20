use crate::error::ForgeResult;
use crate::exec::CommandRunner;
use crate::os::OsPlatform;

/// List top memory-consuming processes (via top)
pub fn mem_hogs_top(runner: &dyn CommandRunner) -> ForgeResult<String> {
    let output = runner.run("top", &["-l", "1", "-o", "rsize"])?;
    if runner.is_dry_run() {
        return Ok(String::new());
    }
    let lines: Vec<&str> = output.stdout.lines().take(20).collect();
    Ok(lines.join("\n"))
}

/// List top memory-consuming processes (via ps)
pub fn mem_hogs_ps(runner: &dyn CommandRunner) -> ForgeResult<String> {
    let output = runner.run("ps", &["wwaxm", "-o", "pid,stat,vsize,rss,time,command"])?;
    if runner.is_dry_run() {
        return Ok(String::new());
    }
    let lines: Vec<&str> = output.stdout.lines().take(10).collect();
    Ok(lines.join("\n"))
}

/// List top CPU-consuming processes
pub fn cpu_hogs(runner: &dyn CommandRunner) -> ForgeResult<String> {
    let output = runner.run("ps", &["wwaxr", "-o", "pid,stat,%cpu,time,command"])?;
    if runner.is_dry_run() {
        return Ok(String::new());
    }
    let lines: Vec<&str> = output.stdout.lines().take(10).collect();
    Ok(lines.join("\n"))
}

/// Snapshot of top CPU consumers (single sample, not continuous)
pub fn top_snapshot(runner: &dyn CommandRunner) -> ForgeResult<String> {
    let output = runner.run("top", &["-l", "1", "-s", "0", "-o", "cpu"])?;
    if runner.is_dry_run() {
        return Ok(String::new());
    }
    Ok(output.stdout)
}

/// Find process by name
pub fn find_pid(runner: &dyn CommandRunner, name: &str) -> ForgeResult<String> {
    let output = runner.run("lsof", &["-t", "-c", name])?;
    Ok(output.stdout)
}

/// List user's processes
pub fn my_ps(runner: &dyn CommandRunner) -> ForgeResult<String> {
    let user = std::env::var("USER").unwrap_or_else(|_| "root".to_string());
    let output = runner.run(
        "ps",
        &[
            "-u",
            &user,
            "-o",
            "pid,%cpu,%mem,start,time,bsdtime,command",
        ],
    )?;
    Ok(output.stdout)
}

/// Show open TCP/IP sockets
pub fn net_cons(runner: &dyn CommandRunner) -> ForgeResult<String> {
    let output = runner.run("lsof", &["-i"])?;
    Ok(output.stdout)
}

/// Get network interface info (routed through dry-run-aware runner)
pub fn ip_info(
    platform: &dyn OsPlatform,
    runner: &dyn CommandRunner,
    interface: &str,
) -> ForgeResult<String> {
    platform.ip_info(runner, interface)
}

/// Show process using a specific port
pub fn used_port(runner: &dyn CommandRunner, port: u16) -> ForgeResult<String> {
    let port_arg = format!("-i4TCP:{port}");
    let output = runner.run("lsof", &["+c", "15", "-nP", &port_arg])?;
    if runner.is_dry_run() {
        return Ok(String::new());
    }
    let lines: Vec<&str> = output
        .stdout
        .lines()
        .filter(|l| l.contains("LISTEN"))
        .collect();
    Ok(lines.join("\n"))
}

/// Get system hardware info (routed through dry-run-aware runner)
pub fn hardware(platform: &dyn OsPlatform, runner: &dyn CommandRunner) -> ForgeResult<String> {
    platform.hardware_info(runner)
}

/// Ping IPv6 multicast
pub fn ping_ipv6(runner: &dyn CommandRunner) -> ForgeResult<String> {
    let output = runner.run("ping6", &["-I", "en0", "ff02::1"])?;
    Ok(output.stdout)
}
