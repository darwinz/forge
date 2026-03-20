use crate::error::ForgeResult;
use crate::exec::CommandRunner;

/// Show the DOCKER_HOST environment variable
pub fn docker_host() -> String {
    match std::env::var("DOCKER_HOST") {
        Ok(val) => format!("DOCKER_HOST={val}"),
        Err(_) => "DOCKER_HOST is not set (using default socket)".to_string(),
    }
}

/// List docker images
pub fn docker_images(runner: &dyn CommandRunner) -> ForgeResult<String> {
    let output = runner.run("docker", &["images"])?;
    if runner.is_dry_run() {
        return Ok(String::new());
    }
    Ok(output.stdout)
}

/// List docker containers
pub fn docker_ps(runner: &dyn CommandRunner) -> ForgeResult<String> {
    let output = runner.run("docker", &["ps", "-a"])?;
    if runner.is_dry_run() {
        return Ok(String::new());
    }
    Ok(output.stdout)
}

/// Remove all docker images (destructive — caller must confirm)
pub fn docker_rm_images(runner: &dyn CommandRunner) -> ForgeResult<String> {
    // First list what will be removed
    let list = runner.run("docker", &["images", "-q"])?;
    if runner.is_dry_run() {
        return Ok(String::new());
    }

    let ids: Vec<&str> = list.stdout.lines().filter(|l| !l.is_empty()).collect();
    if ids.is_empty() {
        return Ok("No images to remove.".to_string());
    }

    let mut args = vec!["rmi"];
    args.extend(&ids);
    let output = runner.run("docker", &args)?;

    if output.success() {
        Ok(format!("Removed {} image(s).", ids.len()))
    } else {
        Ok(format!(
            "Removed some images. Errors:\n{}",
            output.stderr.trim()
        ))
    }
}

/// Remove all docker containers (destructive — caller must confirm)
pub fn docker_rm_containers(runner: &dyn CommandRunner) -> ForgeResult<String> {
    // First list what will be removed
    let list = runner.run("docker", &["ps", "-a", "-q"])?;
    if runner.is_dry_run() {
        return Ok(String::new());
    }

    let ids: Vec<&str> = list.stdout.lines().filter(|l| !l.is_empty()).collect();
    if ids.is_empty() {
        return Ok("No containers to remove.".to_string());
    }

    let mut args = vec!["rm", "-f"];
    args.extend(&ids);
    let output = runner.run("docker", &args)?;

    if output.success() {
        Ok(format!("Removed {} container(s).", ids.len()))
    } else {
        Ok(format!(
            "Removed some containers. Errors:\n{}",
            output.stderr.trim()
        ))
    }
}

/// Remove containers matching a filter (destructive — caller must confirm)
pub fn docker_rm_by_filter(runner: &dyn CommandRunner, filter: &str) -> ForgeResult<String> {
    // List containers matching filter
    let list = runner.run("docker", &["ps", "-a", "-q", "--filter", filter])?;
    if runner.is_dry_run() {
        return Ok(String::new());
    }

    let ids: Vec<&str> = list.stdout.lines().filter(|l| !l.is_empty()).collect();
    if ids.is_empty() {
        return Ok(format!("No containers matching filter '{filter}'."));
    }

    let mut args = vec!["rm", "-f"];
    args.extend(&ids);
    let output = runner.run("docker", &args)?;

    if output.success() {
        Ok(format!(
            "Removed {} container(s) matching '{filter}'.",
            ids.len()
        ))
    } else {
        Ok(format!(
            "Removed some containers. Errors:\n{}",
            output.stderr.trim()
        ))
    }
}
