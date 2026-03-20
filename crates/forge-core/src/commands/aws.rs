//! AWS command implementations.
//!
//! Uses the AWS CLI (`aws`) as the execution backend. All calls go through
//! the CommandRunner abstraction and respect dry-run mode.
//!
//! Security notes:
//! - SSH uses standard OpenSSH host key verification (no StrictHostKeyChecking=no)
//! - No shell interpolation; all arguments are passed as explicit arrays
//! - No implicit key copying or profile mutation

use crate::error::{ForgeError, ForgeResult};
use crate::exec::CommandRunner;

// ---------------------------------------------------------------------------
// Instance model
// ---------------------------------------------------------------------------

/// A resolved EC2 instance.
#[derive(Debug, Clone)]
pub struct Instance {
    pub instance_id: String,
    pub name: String,
    pub state: String,
    pub private_ip: Option<String>,
    pub public_ip: Option<String>,
    pub instance_type: String,
    pub launch_time: String,
}

impl Instance {
    /// Best IP to connect to: public if available, else private.
    pub fn connect_ip(&self) -> Option<&str> {
        self.public_ip.as_deref().or(self.private_ip.as_deref())
    }
}

// ---------------------------------------------------------------------------
// Instance listing
// ---------------------------------------------------------------------------

/// Build AWS CLI base args for profile and region.
fn aws_base_args<'a>(profile: Option<&'a str>, region: Option<&'a str>) -> Vec<&'a str> {
    let mut args = Vec::new();
    if let Some(p) = profile {
        args.push("--profile");
        args.push(p);
    }
    if let Some(r) = region {
        args.push("--region");
        args.push(r);
    }
    args
}

/// List EC2 instances, optionally filtered by Name tag substring.
pub fn list_instances(
    runner: &dyn CommandRunner,
    profile: Option<&str>,
    region: Option<&str>,
    query: Option<&str>,
) -> ForgeResult<Vec<Instance>> {
    let mut args = vec!["ec2", "describe-instances"];
    args.extend(aws_base_args(profile, region));

    // Filter to running/stopped instances; optionally filter by Name tag
    let filter_str;
    if let Some(q) = query {
        filter_str = format!("Name=tag:Name,Values=*{q}*");
        args.push("--filters");
        args.push("Name=instance-state-name,Values=running,stopped");
        args.push("--filters");
        args.push(&filter_str);
    } else {
        args.push("--filters");
        args.push("Name=instance-state-name,Values=running,stopped");
    }

    // Use --query to extract a flat table and --output text for easy parsing
    args.push("--query");
    args.push(
        "Reservations[].Instances[].[\
        InstanceId,\
        Tags[?Key==`Name`].Value|[0],\
        State.Name,\
        PrivateIpAddress,\
        PublicIpAddress,\
        InstanceType,\
        LaunchTime\
    ]",
    );
    args.push("--output");
    args.push("text");

    let output = runner.run("aws", &args)?;

    if runner.is_dry_run() {
        return Ok(Vec::new());
    }

    if !output.success() {
        let msg = output
            .stderr
            .lines()
            .next()
            .unwrap_or("aws ec2 describe-instances failed");
        return Err(ForgeError::CommandFailed(msg.to_string()));
    }

    Ok(parse_instance_output(&output.stdout))
}

/// Parse the tab-separated text output from our --query JMESPath expression.
fn parse_instance_output(stdout: &str) -> Vec<Instance> {
    stdout
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|line| {
            let cols: Vec<&str> = line.split('\t').collect();
            Instance {
                instance_id: cols.first().unwrap_or(&"").to_string(),
                name: cols.get(1).unwrap_or(&"(no name)").to_string(),
                state: cols.get(2).unwrap_or(&"unknown").to_string(),
                private_ip: non_none(cols.get(3)),
                public_ip: non_none(cols.get(4)),
                instance_type: cols.get(5).unwrap_or(&"").to_string(),
                launch_time: cols.get(6).unwrap_or(&"").to_string(),
            }
        })
        .collect()
}

/// Convert AWS "None" placeholder to Option<String>.
fn non_none(val: Option<&&str>) -> Option<String> {
    val.and_then(|v| {
        if v.eq_ignore_ascii_case("none") || v.is_empty() {
            None
        } else {
            Some(v.to_string())
        }
    })
}

/// Format instances for human-readable display.
pub fn format_instances(instances: &[Instance]) -> String {
    if instances.is_empty() {
        return "No instances found.".to_string();
    }

    let mut lines = Vec::new();

    // Header
    lines.push(format!(
        "  {:<20} {:<32} {:<10} {:<16} {:<16} {:<12}",
        "Instance ID", "Name", "State", "Private IP", "Public IP", "Type"
    ));
    lines.push(format!("  {}", "-".repeat(108)));

    for inst in instances {
        lines.push(format!(
            "  {:<20} {:<32} {:<10} {:<16} {:<16} {:<12}",
            inst.instance_id,
            truncate(&inst.name, 30),
            inst.state,
            inst.private_ip.as_deref().unwrap_or("-"),
            inst.public_ip.as_deref().unwrap_or("-"),
            inst.instance_type,
        ));
    }

    lines.join("\n")
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() > max {
        format!("{}…", &s[..max - 1])
    } else {
        s.to_string()
    }
}

// ---------------------------------------------------------------------------
// SSH command construction
// ---------------------------------------------------------------------------

/// Build an SSH command for connecting to an instance.
///
/// Returns the program and args as separate values for the command runner.
/// Uses standard OpenSSH defaults — no StrictHostKeyChecking override.
pub fn build_ssh_command(ip: &str, user: &str, key: Option<&str>) -> (String, Vec<String>) {
    let mut args = Vec::new();

    if let Some(key_path) = key {
        args.push("-i".to_string());
        args.push(key_path.to_string());
    }

    let target = format!("{user}@{ip}");
    args.push(target);

    ("ssh".to_string(), args)
}

/// Execute SSH to an instance (interactive — uses run_piped).
pub fn exec_ssh(
    runner: &dyn CommandRunner,
    ip: &str,
    user: &str,
    key: Option<&str>,
) -> ForgeResult<()> {
    let (program, args) = build_ssh_command(ip, user, key);
    let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    runner.run_piped(&program, &args_refs)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Load balancer instance resolution
// ---------------------------------------------------------------------------

/// List instances behind a load balancer.
///
/// 1. Describe load balancers (optionally filtered by name)
/// 2. Get target group ARNs from listeners
/// 3. Describe target health to get instance IDs
/// 4. Describe those instances
pub fn list_lb_instances(
    runner: &dyn CommandRunner,
    profile: Option<&str>,
    region: Option<&str>,
    lb_name: Option<&str>,
) -> ForgeResult<(String, Vec<Instance>)> {
    // Step 1: Find the load balancer
    let mut args = vec!["elbv2", "describe-load-balancers"];
    args.extend(aws_base_args(profile, region));

    let name_filter;
    if let Some(name) = lb_name {
        name_filter = format!("--names={name}");
        args.push(&name_filter);
    }

    args.push("--query");
    args.push("LoadBalancers[0].[LoadBalancerArn,LoadBalancerName]");
    args.push("--output");
    args.push("text");

    let output = runner.run("aws", &args)?;

    if runner.is_dry_run() {
        return Ok(("(dry-run)".to_string(), Vec::new()));
    }

    if !output.success() || output.stdout.trim().is_empty() || output.stdout.trim() == "None" {
        return Err(ForgeError::CommandFailed(
            "No load balancer found. Check the name and profile.".to_string(),
        ));
    }

    let lb_parts: Vec<&str> = output.stdout.trim().split('\t').collect();
    let lb_arn = lb_parts
        .first()
        .ok_or_else(|| ForgeError::CommandFailed("Could not parse LB ARN".to_string()))?;
    let lb_display_name = lb_parts.get(1).unwrap_or(&"unknown").to_string();

    // Step 2: Get target group ARNs from listeners
    let mut args2 = vec!["elbv2", "describe-listeners"];
    args2.extend(aws_base_args(profile, region));
    args2.push("--load-balancer-arn");
    args2.push(lb_arn);
    args2.push("--query");
    args2.push("Listeners[].DefaultActions[].TargetGroupArn");
    args2.push("--output");
    args2.push("text");

    let output2 = runner.run("aws", &args2)?;

    if !output2.success() || output2.stdout.trim().is_empty() {
        return Err(ForgeError::CommandFailed(
            "Could not list target groups for load balancer.".to_string(),
        ));
    }

    let tg_arn = output2
        .stdout
        .split_whitespace()
        .find(|s| s.contains("targetgroup"))
        .ok_or_else(|| {
            ForgeError::CommandFailed("No target group found for load balancer.".to_string())
        })?;

    // Step 3: Get instance IDs from target health
    let mut args3 = vec!["elbv2", "describe-target-health"];
    args3.extend(aws_base_args(profile, region));
    args3.push("--target-group-arn");
    args3.push(tg_arn);
    args3.push("--query");
    args3.push("TargetHealthDescriptions[].Target.Id");
    args3.push("--output");
    args3.push("text");

    let output3 = runner.run("aws", &args3)?;

    if !output3.success() || output3.stdout.trim().is_empty() {
        return Ok((lb_display_name, Vec::new()));
    }

    let instance_ids: Vec<&str> = output3
        .stdout
        .split_whitespace()
        .filter(|s| s.starts_with("i-"))
        .collect();

    if instance_ids.is_empty() {
        return Ok((lb_display_name, Vec::new()));
    }

    // Step 4: Describe those instances
    let mut args4 = vec!["ec2", "describe-instances"];
    args4.extend(aws_base_args(profile, region));
    args4.push("--instance-ids");
    for id in &instance_ids {
        args4.push(id);
    }
    args4.push("--query");
    args4.push(
        "Reservations[].Instances[].[\
        InstanceId,\
        Tags[?Key==`Name`].Value|[0],\
        State.Name,\
        PrivateIpAddress,\
        PublicIpAddress,\
        InstanceType,\
        LaunchTime\
    ]",
    );
    args4.push("--output");
    args4.push("text");

    let output4 = runner.run("aws", &args4)?;

    let instances = if output4.success() {
        parse_instance_output(&output4.stdout)
    } else {
        Vec::new()
    };

    Ok((lb_display_name, instances))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_instance_output_basic() {
        let input =
            "i-abc123\tmy-server\trunning\t10.0.1.5\t54.1.2.3\tt3.micro\t2024-01-15T10:00:00Z\n";
        let instances = parse_instance_output(input);
        assert_eq!(instances.len(), 1);
        assert_eq!(instances[0].instance_id, "i-abc123");
        assert_eq!(instances[0].name, "my-server");
        assert_eq!(instances[0].state, "running");
        assert_eq!(instances[0].private_ip.as_deref(), Some("10.0.1.5"));
        assert_eq!(instances[0].public_ip.as_deref(), Some("54.1.2.3"));
        assert_eq!(instances[0].instance_type, "t3.micro");
    }

    #[test]
    fn test_parse_instance_output_none_ips() {
        let input = "i-abc123\tweb-01\trunning\t10.0.1.5\tNone\tt3.medium\t2024-01-15\n";
        let instances = parse_instance_output(input);
        assert_eq!(instances.len(), 1);
        assert_eq!(instances[0].private_ip.as_deref(), Some("10.0.1.5"));
        assert!(instances[0].public_ip.is_none());
    }

    #[test]
    fn test_parse_instance_output_empty() {
        assert!(parse_instance_output("").is_empty());
        assert!(parse_instance_output("  \n  \n").is_empty());
    }

    #[test]
    fn test_parse_instance_output_multiple() {
        let input = "\
i-aaa\tserver-a\trunning\t10.0.0.1\t1.2.3.4\tt3.micro\t2024-01-01
i-bbb\tserver-b\tstopped\t10.0.0.2\tNone\tt3.small\t2024-02-01
";
        let instances = parse_instance_output(input);
        assert_eq!(instances.len(), 2);
        assert_eq!(instances[0].name, "server-a");
        assert_eq!(instances[1].name, "server-b");
        assert_eq!(instances[1].state, "stopped");
    }

    #[test]
    fn test_connect_ip_prefers_public() {
        let inst = Instance {
            instance_id: "i-1".into(),
            name: "test".into(),
            state: "running".into(),
            private_ip: Some("10.0.0.1".into()),
            public_ip: Some("54.1.2.3".into()),
            instance_type: "t3.micro".into(),
            launch_time: "".into(),
        };
        assert_eq!(inst.connect_ip(), Some("54.1.2.3"));
    }

    #[test]
    fn test_connect_ip_falls_back_to_private() {
        let inst = Instance {
            instance_id: "i-1".into(),
            name: "test".into(),
            state: "running".into(),
            private_ip: Some("10.0.0.1".into()),
            public_ip: None,
            instance_type: "t3.micro".into(),
            launch_time: "".into(),
        };
        assert_eq!(inst.connect_ip(), Some("10.0.0.1"));
    }

    #[test]
    fn test_connect_ip_none() {
        let inst = Instance {
            instance_id: "i-1".into(),
            name: "test".into(),
            state: "running".into(),
            private_ip: None,
            public_ip: None,
            instance_type: "t3.micro".into(),
            launch_time: "".into(),
        };
        assert!(inst.connect_ip().is_none());
    }

    #[test]
    fn test_build_ssh_command_basic() {
        let (prog, args) = build_ssh_command("54.1.2.3", "ubuntu", None);
        assert_eq!(prog, "ssh");
        assert_eq!(args, vec!["ubuntu@54.1.2.3"]);
    }

    #[test]
    fn test_build_ssh_command_with_key() {
        let (prog, args) = build_ssh_command("10.0.0.1", "ec2-user", Some("~/.ssh/mykey.pem"));
        assert_eq!(prog, "ssh");
        assert_eq!(args, vec!["-i", "~/.ssh/mykey.pem", "ec2-user@10.0.0.1"]);
    }

    #[test]
    fn test_build_ssh_no_strict_host_checking_override() {
        let (_, args) = build_ssh_command("10.0.0.1", "ubuntu", None);
        // Must NOT contain any StrictHostKeyChecking override
        for arg in &args {
            assert!(
                !arg.contains("StrictHostKeyChecking"),
                "SSH command must not override host key checking"
            );
        }
    }

    #[test]
    fn test_format_instances_empty() {
        assert_eq!(format_instances(&[]), "No instances found.");
    }

    #[test]
    fn test_format_instances_table() {
        let instances = vec![Instance {
            instance_id: "i-abc123".into(),
            name: "my-server".into(),
            state: "running".into(),
            private_ip: Some("10.0.1.5".into()),
            public_ip: Some("54.1.2.3".into()),
            instance_type: "t3.micro".into(),
            launch_time: "2024-01-15".into(),
        }];
        let output = format_instances(&instances);
        assert!(output.contains("i-abc123"));
        assert!(output.contains("my-server"));
        assert!(output.contains("10.0.1.5"));
        assert!(output.contains("54.1.2.3"));
        assert!(output.contains("Instance ID"));
    }

    #[test]
    fn test_non_none() {
        assert_eq!(non_none(Some(&"10.0.0.1")), Some("10.0.0.1".to_string()));
        assert_eq!(non_none(Some(&"None")), None);
        assert_eq!(non_none(Some(&"none")), None);
        assert_eq!(non_none(Some(&"")), None);
        assert_eq!(non_none(None), None);
    }

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("short", 10), "short");
        assert_eq!(truncate("a-very-long-name", 10), "a-very-lo…");
    }

    #[test]
    fn test_aws_base_args() {
        let args = aws_base_args(None, None);
        assert!(args.is_empty());

        let args = aws_base_args(Some("prod"), None);
        assert_eq!(args, vec!["--profile", "prod"]);

        let args = aws_base_args(Some("dev"), Some("us-west-2"));
        assert_eq!(args, vec!["--profile", "dev", "--region", "us-west-2"]);
    }
}
