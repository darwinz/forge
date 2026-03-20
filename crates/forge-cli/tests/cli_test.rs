use assert_cmd::Command;
use predicates::prelude::*;

fn forge() -> Command {
    Command::cargo_bin("forge").unwrap()
}

#[test]
fn test_help() {
    forge()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Developer toolkit"))
        .stdout(predicate::str::contains("notes"))
        .stdout(predicate::str::contains("system"))
        .stdout(predicate::str::contains("bootstrap"))
        .stdout(predicate::str::contains("skill"));
}

#[test]
fn test_version() {
    forge()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("forge"));
}

#[test]
fn test_notes_list() {
    forge()
        .arg("notes")
        .assert()
        .success()
        .stdout(predicate::str::contains("Available notes topics"))
        .stdout(predicate::str::contains("k8s"))
        .stdout(predicate::str::contains("terraform"))
        .stdout(predicate::str::contains("screen"))
        .stdout(predicate::str::contains("claude"))
        .stdout(predicate::str::contains("codex"));
}

#[test]
fn test_notes_k8s() {
    forge()
        .args(["notes", "k8s"])
        .assert()
        .success()
        .stdout(predicate::str::contains("kubectl"));
}

#[test]
fn test_notes_terraform() {
    forge()
        .args(["notes", "terraform"])
        .assert()
        .success()
        .stdout(predicate::str::contains("terraform init"))
        .stdout(predicate::str::contains("terraform plan"));
}

#[test]
fn test_notes_claude() {
    forge()
        .args(["notes", "claude"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Claude Code"))
        .stdout(predicate::str::contains("claude -p"))
        .stdout(predicate::str::contains("--model"))
        .stdout(predicate::str::contains("--permission-mode"));
}

#[test]
fn test_notes_codex() {
    forge()
        .args(["notes", "codex"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Codex CLI"))
        .stdout(predicate::str::contains("codex exec"))
        .stdout(predicate::str::contains("--full-auto"))
        .stdout(predicate::str::contains("codex review"));
}

#[test]
fn test_notes_nvim() {
    forge()
        .args(["notes", "nvim"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Leader: ,"))
        .stdout(predicate::str::contains("Telescope"))
        .stdout(predicate::str::contains("go.nvim"))
        .stdout(predicate::str::contains(",gs"))
        .stdout(predicate::str::contains("lazy.nvim"));
}

#[test]
fn test_notes_list_includes_nvim() {
    forge()
        .arg("notes")
        .assert()
        .success()
        .stdout(predicate::str::contains("nvim"));
}

// ---------------------------------------------------------------------------
// Notes discovery UX
// ---------------------------------------------------------------------------

#[test]
fn test_notes_list_subcommand() {
    // `forge notes list` should behave like `forge notes`
    forge()
        .args(["notes", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Available notes topics"))
        .stdout(predicate::str::contains("k8s"))
        .stdout(predicate::str::contains("Usage: forge notes"));
}

#[test]
fn test_notes_help_subcommand() {
    // `forge notes help` should show help (clap built-in)
    forge()
        .args(["notes", "help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("notes"))
        .stdout(predicate::str::contains("list"));
}

#[test]
fn test_notes_shows_usage_hint() {
    // Bare `forge notes` should include a usage hint
    forge()
        .arg("notes")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage: forge notes"));
}

#[test]
fn test_notes_invalid_topic() {
    forge().args(["notes", "nonexistent"]).assert().failure();
}

// ---------------------------------------------------------------------------
// Subcommand groups show help when called bare
// ---------------------------------------------------------------------------

#[test]
fn test_system_bare_shows_help() {
    // `forge system` with no subcommand should show help
    forge()
        .arg("system")
        .assert()
        .failure() // arg_required_else_help exits with error code
        .stderr(predicate::str::contains("Usage"));
}

#[test]
fn test_system_help_subcommand() {
    // `forge system help` should list subcommands
    forge()
        .args(["system", "help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("cpu-hogs"))
        .stdout(predicate::str::contains("hardware"));
}

#[test]
fn test_system_help_port() {
    // `forge system help port` should show port-specific help
    forge()
        .args(["system", "help", "port"])
        .assert()
        .success()
        .stdout(predicate::str::contains("PORT"))
        .stdout(predicate::str::contains("Port number"));
}

#[test]
fn test_skill_bare_shows_help() {
    forge()
        .arg("skill")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Usage"));
}

#[test]
fn test_file_bare_shows_help() {
    forge()
        .arg("file")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Usage"));
}

#[test]
fn test_misc_bare_shows_help() {
    forge()
        .arg("misc")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Usage"));
}

#[test]
fn test_system_help() {
    forge()
        .args(["system", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("cpu-hogs"))
        .stdout(predicate::str::contains("hardware"));
}

#[test]
fn test_dry_run_cpu_hogs() {
    forge()
        .args(["--dry-run", "system", "cpu-hogs"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[dry-run]"));
}

#[test]
fn test_bootstrap_list_bundles() {
    forge()
        .args(["bootstrap", "--list-bundles"])
        .assert()
        .success()
        .stdout(predicate::str::contains("core"))
        .stdout(predicate::str::contains("go"))
        .stdout(predicate::str::contains("node"));
}

#[test]
fn test_skill_list() {
    forge().args(["skill", "list"]).assert().success();
}

#[test]
fn test_skill_validate_all() {
    forge()
        .args(["skill", "validate", "--all"])
        .assert()
        .success();
}

#[test]
fn test_skill_audit() {
    forge()
        .args(["skill", "audit"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No audit records"));
}

#[test]
fn test_bootstrap_scan() {
    forge()
        .args(["bootstrap", "--scan"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Environment scan"));
}

#[test]
fn test_bootstrap_help_mentions_scan() {
    forge()
        .args(["bootstrap", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--scan"))
        .stdout(predicate::str::contains("--status"))
        .stdout(predicate::str::contains("--list-bundles"));
}

// ---------------------------------------------------------------------------
// Regression: dry-run must not execute OS commands
// ---------------------------------------------------------------------------

#[test]
fn test_dry_run_system_hardware() {
    // Must print dry-run messages for system_profiler/diskutil, not execute them
    forge()
        .args(["--dry-run", "system", "hardware"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[dry-run]"));
}

#[test]
fn test_dry_run_system_ip_info() {
    // Must print dry-run message for ipconfig, not execute it
    forge()
        .args(["--dry-run", "system", "ip-info"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[dry-run]"));
}

#[test]
fn test_dry_run_system_top() {
    forge()
        .args(["--dry-run", "system", "top"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[dry-run]"));
}

// ---------------------------------------------------------------------------
// Regression: dry-run bootstrap --status shows unknown, not missing
// ---------------------------------------------------------------------------

#[test]
fn test_dry_run_bootstrap_status() {
    forge()
        .args(["--dry-run", "bootstrap", "--status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("dry-run"))
        .stdout(predicate::str::contains("[unknown]"))
        // Must NOT claim things are missing when we didn't actually check
        .stdout(predicate::str::contains("[missing]").not());
}

// ---------------------------------------------------------------------------
// Regression: system top description is accurate
// ---------------------------------------------------------------------------

#[test]
fn test_system_top_description() {
    forge()
        .args(["system", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("snapshot"))
        // Must NOT say "continuous"
        .stdout(predicate::str::contains("ontinuous").not());
}

// ---------------------------------------------------------------------------
// Shell alias generation
// ---------------------------------------------------------------------------

#[test]
fn test_shell_generate_aliases() {
    forge()
        .args(["shell", "generate-aliases"])
        .assert()
        .success()
        .stdout(predicate::str::contains("#!/bin/sh"))
        .stdout(predicate::str::contains("Generated by forge"))
        .stdout(predicate::str::contains("alias ll="))
        .stdout(predicate::str::contains("alias k="))
        .stdout(predicate::str::contains("flushDNS"));
}

// ---------------------------------------------------------------------------
// File commands (read-only)
// ---------------------------------------------------------------------------

#[test]
fn test_file_find_dry_run() {
    forge()
        .args(["--dry-run", "file", "find", "*.rs"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[dry-run]"));
}

#[test]
fn test_file_search_dry_run() {
    forge()
        .args(["--dry-run", "file", "search", "TODO"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[dry-run]"));
}

#[test]
fn test_file_spotlight_dry_run() {
    forge()
        .args(["--dry-run", "file", "spotlight", "test"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[dry-run]"));
}

// ---------------------------------------------------------------------------
// Misc commands
// ---------------------------------------------------------------------------

#[test]
fn test_misc_weather_dry_run() {
    forge()
        .args(["--dry-run", "misc", "weather"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[dry-run]"));
}

#[test]
fn test_misc_define_dry_run() {
    forge()
        .args(["--dry-run", "misc", "define", "hello"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[dry-run]"));
}

// ---------------------------------------------------------------------------
// Bootstrap --status --format json
// ---------------------------------------------------------------------------

#[test]
fn test_bootstrap_status_json() {
    forge()
        .args(["bootstrap", "--status", "--format", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"bundles\""))
        .stdout(predicate::str::contains("\"name\""))
        .stdout(predicate::str::contains("\"state\""));
}

#[test]
fn test_dry_run_bootstrap_status_json() {
    forge()
        .args(["--dry-run", "bootstrap", "--status", "--format", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"state\": \"unknown\""));
}

// ---------------------------------------------------------------------------
// Bootstrap install planning
// ---------------------------------------------------------------------------

#[test]
fn test_bootstrap_dry_run_shows_plan() {
    // `forge --dry-run bootstrap --bundles core` should show the plan without executing
    forge()
        .args(["--dry-run", "bootstrap", "--bundles", "core"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Bootstrap plan"))
        .stdout(predicate::str::contains("[dry-run]"));
}

#[test]
fn test_bootstrap_dry_run_specific_bundle() {
    forge()
        .args(["--dry-run", "bootstrap", "--bundles", "ai-tools"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Bootstrap plan"))
        // ai-tools has manual entries (claude, firecrawl-cli)
        .stdout(predicate::str::contains("Manual install required"));
}

#[test]
fn test_bootstrap_dry_run_add_bundle() {
    forge()
        .args(["--dry-run", "bootstrap", "--add", "core"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Bootstrap plan"));
}

#[test]
fn test_bootstrap_dry_run_all() {
    forge()
        .args(["--dry-run", "bootstrap", "--all"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Bootstrap plan"))
        .stdout(predicate::str::contains("[dry-run]"));
}

#[test]
fn test_bootstrap_unknown_bundle() {
    forge()
        .args(["--dry-run", "bootstrap", "--bundles", "nonexistent"])
        .assert()
        .success()
        .stderr(predicate::str::contains("Unknown bundle"));
}

#[test]
fn test_bootstrap_default_profile() {
    // `forge --dry-run bootstrap` with no bundle args loads the default profile
    forge()
        .args(["--dry-run", "bootstrap"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Bootstrap plan"));
}

#[test]
fn test_bootstrap_help_shows_yes_flag() {
    forge()
        .args(["bootstrap", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--yes"))
        .stdout(predicate::str::contains("-y"));
}

#[test]
fn test_bootstrap_manual_entries_show_instructions() {
    // ai-tools bundle has manual entries — verify instructions appear
    forge()
        .args(["--dry-run", "bootstrap", "--bundles", "ai-tools"])
        .assert()
        .success()
        .stdout(predicate::str::contains("claude"))
        .stdout(predicate::str::contains("Install:"));
}

// ---------------------------------------------------------------------------
// File mutation commands
// ---------------------------------------------------------------------------

#[test]
fn test_file_help_shows_all_commands() {
    forge()
        .args(["file", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("find"))
        .stdout(predicate::str::contains("search"))
        .stdout(predicate::str::contains("extract"))
        .stdout(predicate::str::contains("trash"))
        .stdout(predicate::str::contains("cleanup-ds"));
}

#[test]
fn test_file_extract_dry_run() {
    forge()
        .args(["--dry-run", "file", "extract", "test.tar.gz"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[dry-run]"))
        .stdout(predicate::str::contains("tar"));
}

#[test]
fn test_file_extract_unsupported_format() {
    forge()
        .args(["--dry-run", "file", "extract", "test.txt"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Unsupported archive format"));
}

#[test]
fn test_file_trash_dry_run() {
    forge()
        .args(["--dry-run", "file", "trash", "somefile.txt"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[dry-run]"));
}

#[test]
fn test_file_cleanup_ds_dry_run() {
    forge()
        .args(["--dry-run", "file", "cleanup-ds"])
        .assert()
        .success();
    // In dry-run, either "No .DS_Store files found" or "[dry-run]"
}

#[test]
fn test_file_cleanup_ds_has_yes_flag() {
    forge()
        .args(["file", "cleanup-ds", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--yes"))
        .stdout(predicate::str::contains("-y"));
}

// ---------------------------------------------------------------------------
// Docker commands
// ---------------------------------------------------------------------------

#[test]
fn test_docker_bare_shows_help() {
    forge()
        .arg("docker")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Usage"));
}

#[test]
fn test_docker_help() {
    forge()
        .args(["docker", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("host"))
        .stdout(predicate::str::contains("images"))
        .stdout(predicate::str::contains("ps"))
        .stdout(predicate::str::contains("rm-images"))
        .stdout(predicate::str::contains("rm-containers"))
        .stdout(predicate::str::contains("rm-by-filter"));
}

#[test]
fn test_docker_host() {
    // Should always succeed — just shows DOCKER_HOST or "not set"
    forge()
        .args(["docker", "host"])
        .assert()
        .success()
        .stdout(predicate::str::contains("DOCKER_HOST"));
}

#[test]
fn test_docker_images_dry_run() {
    forge()
        .args(["--dry-run", "docker", "images"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[dry-run]"))
        .stdout(predicate::str::contains("docker images"));
}

#[test]
fn test_docker_ps_dry_run() {
    forge()
        .args(["--dry-run", "docker", "ps"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[dry-run]"))
        .stdout(predicate::str::contains("docker ps"));
}

#[test]
fn test_docker_rm_images_dry_run() {
    forge()
        .args(["--dry-run", "docker", "rm-images"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[dry-run]"))
        .stdout(predicate::str::contains("docker images"));
}

#[test]
fn test_docker_rm_containers_dry_run() {
    forge()
        .args(["--dry-run", "docker", "rm-containers"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[dry-run]"))
        .stdout(predicate::str::contains("docker ps"));
}

#[test]
fn test_docker_rm_by_filter_dry_run() {
    forge()
        .args(["--dry-run", "docker", "rm-by-filter", "status=exited"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[dry-run]"))
        .stdout(predicate::str::contains("docker ps"));
}

#[test]
fn test_docker_rm_images_has_yes_flag() {
    forge()
        .args(["docker", "rm-images", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--yes"))
        .stdout(predicate::str::contains("-y"));
}

#[test]
fn test_docker_rm_containers_has_yes_flag() {
    forge()
        .args(["docker", "rm-containers", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--yes"))
        .stdout(predicate::str::contains("-y"));
}

#[test]
fn test_docker_rm_by_filter_has_yes_flag() {
    forge()
        .args(["docker", "rm-by-filter", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--yes"))
        .stdout(predicate::str::contains("-y"));
}

// ---------------------------------------------------------------------------
// Git commands
// ---------------------------------------------------------------------------

#[test]
fn test_git_bare_shows_help() {
    forge()
        .arg("git")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Usage"));
}

#[test]
fn test_git_help() {
    forge()
        .args(["git", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("setup-aliases"))
        .stdout(predicate::str::contains("list-aliases"));
}

#[test]
fn test_git_list_aliases() {
    forge()
        .args(["git", "list-aliases"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Configured git aliases"))
        .stdout(predicate::str::contains("git co"))
        .stdout(predicate::str::contains("checkout"));
}

#[test]
fn test_git_list_aliases_shows_source() {
    forge()
        .args(["git", "list-aliases"])
        .assert()
        .success()
        .stdout(predicate::str::contains("git-aliases.toml"));
}

#[test]
fn test_git_setup_aliases_dry_run() {
    forge()
        .args(["--dry-run", "git", "setup-aliases"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[dry-run]"))
        .stdout(predicate::str::contains("preview"));
}

#[test]
fn test_git_setup_aliases_dry_run_shows_preview() {
    // In dry-run mode, git config isn't queried, so all aliases show as "to add"
    forge()
        .args(["--dry-run", "git", "setup-aliases"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Will add"))
        .stdout(predicate::str::contains("[dry-run]"));
}

// ---------------------------------------------------------------------------
// Bootstrap — go and gem source support
// ---------------------------------------------------------------------------

#[test]
fn test_bootstrap_dry_run_go_bundle() {
    // go bundle has go tools and brew formulae — all should appear in plan
    forge()
        .args(["--dry-run", "bootstrap", "--bundles", "go"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Bootstrap plan"))
        .stdout(predicate::str::contains("gopls"))
        .stdout(predicate::str::contains("[dry-run]"));
}

#[test]
fn test_bootstrap_dry_run_go_tools_show_as_installable() {
    // go tools should appear in "Will install", not "Skipped"
    forge()
        .args(["--dry-run", "bootstrap", "--bundles", "go"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Will install"))
        // Should NOT show "go installation not yet supported"
        .stdout(predicate::str::contains("go installation not yet supported").not());
}

#[test]
fn test_bootstrap_dry_run_all_sources_represented() {
    // --all includes bundles with brew, npm, go, manual — verify plan output
    forge()
        .args(["--dry-run", "bootstrap", "--all"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Bootstrap plan"))
        .stdout(predicate::str::contains("[dry-run]"))
        .stdout(predicate::str::contains("[go]").or(predicate::str::contains("go")));
}

#[test]
fn test_bootstrap_dry_run_no_install_without_confirmation() {
    // dry-run should show plan and exit without installing
    forge()
        .args(["--dry-run", "bootstrap", "--bundles", "core"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[dry-run]"))
        // Must NOT contain "Installing..." which would indicate execution
        .stdout(predicate::str::contains("Installing...").not());
}

#[test]
fn test_bootstrap_plan_groups_by_source() {
    // Plan output should show source labels for each package
    forge()
        .args(["--dry-run", "bootstrap", "--bundles", "go"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[brew]").or(predicate::str::contains("[go]")));
}

// ---------------------------------------------------------------------------
// AWS commands
// ---------------------------------------------------------------------------

#[test]
fn test_aws_bare_shows_help() {
    forge()
        .arg("aws")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Usage"));
}

#[test]
fn test_aws_help() {
    forge()
        .args(["aws", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("instances"))
        .stdout(predicate::str::contains("ssh"))
        .stdout(predicate::str::contains("connect"));
}

#[test]
fn test_aws_instances_help() {
    forge()
        .args(["aws", "instances", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--profile"))
        .stdout(predicate::str::contains("--query"))
        .stdout(predicate::str::contains("--region"));
}

#[test]
fn test_aws_ssh_help() {
    forge()
        .args(["aws", "ssh", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--profile"))
        .stdout(predicate::str::contains("--query"))
        .stdout(predicate::str::contains("--key"))
        .stdout(predicate::str::contains("--user"))
        .stdout(predicate::str::contains("--region"));
}

#[test]
fn test_aws_connect_help() {
    forge()
        .args(["aws", "connect", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--profile"))
        .stdout(predicate::str::contains("--lb-name"))
        .stdout(predicate::str::contains("--region"));
}

#[test]
fn test_aws_instances_dry_run() {
    forge()
        .args(["--dry-run", "aws", "instances"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[dry-run]"));
}

#[test]
fn test_aws_instances_dry_run_with_profile() {
    forge()
        .args(["--dry-run", "aws", "instances", "--profile", "dev"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[dry-run]"))
        .stdout(predicate::str::contains("--profile"))
        .stdout(predicate::str::contains("dev"));
}

#[test]
fn test_aws_instances_dry_run_with_query() {
    forge()
        .args(["--dry-run", "aws", "instances", "--query", "web"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[dry-run]"));
}

#[test]
fn test_aws_ssh_dry_run() {
    forge()
        .args(["--dry-run", "aws", "ssh"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[dry-run]"));
}

#[test]
fn test_aws_connect_dry_run() {
    forge()
        .args(["--dry-run", "aws", "connect"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[dry-run]"));
}

// ---------------------------------------------------------------------------
// Platform commands
// ---------------------------------------------------------------------------

#[test]
fn test_platform_bare_shows_help() {
    forge()
        .arg("platform")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Usage"));
}

#[test]
fn test_platform_help() {
    forge()
        .args(["platform", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("doctor"))
        .stdout(predicate::str::contains("status"));
}

#[test]
fn test_platform_status() {
    // Should always succeed — reports installed/missing for each platform
    forge()
        .args(["platform", "status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Platform CLI status"))
        .stdout(predicate::str::contains("Vercel"))
        .stdout(predicate::str::contains("Supabase"))
        .stdout(predicate::str::contains("Netlify"))
        .stdout(predicate::str::contains("Render"))
        .stdout(predicate::str::contains("Appwrite"));
}

#[test]
fn test_platform_status_dry_run() {
    forge()
        .args(["--dry-run", "platform", "status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Platform CLI status"))
        // In dry-run, all should show as missing since we can't check
        .stdout(predicate::str::contains("[missing]"));
}

#[test]
fn test_platform_doctor() {
    forge()
        .args(["platform", "doctor"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Platform doctor"));
}

#[test]
fn test_platform_doctor_dry_run() {
    forge()
        .args(["--dry-run", "platform", "doctor"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Platform doctor"))
        .stdout(predicate::str::contains("not installed"));
}

// ---------------------------------------------------------------------------
// Notes — platform topics
// ---------------------------------------------------------------------------

#[test]
fn test_notes_vercel() {
    forge()
        .args(["notes", "vercel"])
        .assert()
        .success()
        .stdout(predicate::str::contains("vercel login"))
        .stdout(predicate::str::contains("vercel deploy"))
        .stdout(predicate::str::contains("vercel env"));
}

#[test]
fn test_notes_supabase() {
    forge()
        .args(["notes", "supabase"])
        .assert()
        .success()
        .stdout(predicate::str::contains("supabase start"))
        .stdout(predicate::str::contains("supabase db"));
}

#[test]
fn test_notes_netlify() {
    forge()
        .args(["notes", "netlify"])
        .assert()
        .success()
        .stdout(predicate::str::contains("netlify deploy"))
        .stdout(predicate::str::contains("netlify dev"));
}

#[test]
fn test_notes_render() {
    forge()
        .args(["notes", "render"])
        .assert()
        .success()
        .stdout(predicate::str::contains("render services"))
        .stdout(predicate::str::contains("render deploys"));
}

#[test]
fn test_notes_appwrite() {
    forge()
        .args(["notes", "appwrite"])
        .assert()
        .success()
        .stdout(predicate::str::contains("appwrite deploy"))
        .stdout(predicate::str::contains("appwrite functions"));
}

#[test]
fn test_notes_list_includes_platforms() {
    forge()
        .arg("notes")
        .assert()
        .success()
        .stdout(predicate::str::contains("vercel"))
        .stdout(predicate::str::contains("supabase"))
        .stdout(predicate::str::contains("netlify"))
        .stdout(predicate::str::contains("render"))
        .stdout(predicate::str::contains("appwrite"));
}

// ---------------------------------------------------------------------------
// Bundle: platforms bundle exists
// ---------------------------------------------------------------------------

#[test]
fn test_bootstrap_list_includes_platforms() {
    forge()
        .args(["bootstrap", "--list-bundles"])
        .assert()
        .success()
        .stdout(predicate::str::contains("platforms"));
}

// ---------------------------------------------------------------------------
// Vercel commands
// ---------------------------------------------------------------------------

#[test]
fn test_vercel_bare_shows_help() {
    forge()
        .arg("vercel")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Usage"));
}

#[test]
fn test_vercel_help() {
    forge()
        .args(["vercel", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("doctor"))
        .stdout(predicate::str::contains("env-diff"))
        .stdout(predicate::str::contains("deploy"));
}

#[test]
fn test_vercel_doctor() {
    forge()
        .args(["vercel", "doctor"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Vercel doctor"));
}

#[test]
fn test_vercel_doctor_dry_run() {
    forge()
        .args(["--dry-run", "vercel", "doctor"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Vercel doctor"))
        .stdout(predicate::str::contains("[dry-run]"));
}

#[test]
fn test_vercel_env_diff() {
    forge()
        .args(["vercel", "env-diff"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Vercel env diff"));
}

#[test]
fn test_vercel_env_diff_dry_run() {
    forge()
        .args(["--dry-run", "vercel", "env-diff"])
        .assert()
        .success()
        .stdout(predicate::str::contains("env diff"))
        .stdout(predicate::str::contains("not available"));
}

#[test]
fn test_vercel_deploy_dry_run() {
    forge()
        .args(["--dry-run", "vercel", "deploy"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[dry-run]"))
        .stdout(predicate::str::contains("vercel deploy"));
}

#[test]
fn test_vercel_deploy_prod_dry_run() {
    forge()
        .args(["--dry-run", "vercel", "deploy", "--prod"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[dry-run]"))
        .stdout(predicate::str::contains("--prod"));
}

#[test]
fn test_vercel_deploy_has_yes_flag() {
    forge()
        .args(["vercel", "deploy", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--yes"))
        .stdout(predicate::str::contains("-y"))
        .stdout(predicate::str::contains("--prod"));
}

// ---------------------------------------------------------------------------
// Supabase commands
// ---------------------------------------------------------------------------

#[test]
fn test_supabase_bare_shows_help() {
    forge()
        .arg("supabase")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Usage"));
}

#[test]
fn test_supabase_help() {
    forge()
        .args(["supabase", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("doctor"))
        .stdout(predicate::str::contains("migration-status"))
        .stdout(predicate::str::contains("reset"));
}

#[test]
fn test_supabase_doctor() {
    forge()
        .args(["supabase", "doctor"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Supabase doctor"));
}

#[test]
fn test_supabase_doctor_dry_run() {
    forge()
        .args(["--dry-run", "supabase", "doctor"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Supabase doctor"))
        .stdout(predicate::str::contains("[dry-run]"));
}

#[test]
fn test_supabase_migration_status() {
    forge()
        .args(["supabase", "migration-status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("migration status"));
}

#[test]
fn test_supabase_migration_status_dry_run() {
    forge()
        .args(["--dry-run", "supabase", "migration-status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("migration status"))
        .stdout(predicate::str::contains("[dry-run]"));
}

#[test]
fn test_supabase_reset_dry_run() {
    forge()
        .args(["--dry-run", "supabase", "reset"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[dry-run]"))
        .stdout(predicate::str::contains("supabase db reset"));
}

#[test]
fn test_supabase_reset_has_yes_flag() {
    forge()
        .args(["supabase", "reset", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--yes"))
        .stdout(predicate::str::contains("-y"));
}

#[test]
fn test_supabase_services_help() {
    forge()
        .args(["supabase", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("services"));
}

#[test]
fn test_supabase_services_dry_run() {
    forge()
        .args(["--dry-run", "supabase", "services"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[dry-run]"));
}

// ---------------------------------------------------------------------------
// Vercel — status command
// ---------------------------------------------------------------------------

#[test]
fn test_vercel_status() {
    forge()
        .args(["vercel", "status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Vercel deployments"));
}

#[test]
fn test_vercel_status_dry_run() {
    forge()
        .args(["--dry-run", "vercel", "status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[dry-run]"));
}

#[test]
fn test_vercel_help_includes_status() {
    forge()
        .args(["vercel", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("status"))
        .stdout(predicate::str::contains("doctor"))
        .stdout(predicate::str::contains("env-diff"))
        .stdout(predicate::str::contains("deploy"));
}

// ---------------------------------------------------------------------------
// Netlify commands
// ---------------------------------------------------------------------------

#[test]
fn test_netlify_bare_shows_help() {
    forge()
        .arg("netlify")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Usage"));
}

#[test]
fn test_netlify_help() {
    forge()
        .args(["netlify", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("doctor"))
        .stdout(predicate::str::contains("env-diff"));
}

#[test]
fn test_netlify_doctor() {
    forge()
        .args(["netlify", "doctor"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Netlify doctor"));
}

#[test]
fn test_netlify_doctor_dry_run() {
    forge()
        .args(["--dry-run", "netlify", "doctor"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Netlify doctor"))
        .stdout(predicate::str::contains("[dry-run]"));
}

#[test]
fn test_netlify_env_diff() {
    forge()
        .args(["netlify", "env-diff"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Netlify env diff"));
}

#[test]
fn test_netlify_env_diff_dry_run() {
    forge()
        .args(["--dry-run", "netlify", "env-diff"])
        .assert()
        .success()
        .stdout(predicate::str::contains("env diff"))
        .stdout(predicate::str::contains("not available"));
}
