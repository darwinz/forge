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
    forge()
        .args(["notes", "nonexistent"])
        .assert()
        .failure();
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
    forge()
        .args(["skill", "list"])
        .assert()
        .success();
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
// Unimplemented stubs
// ---------------------------------------------------------------------------

#[test]
fn test_unimplemented_commands_show_message() {
    forge()
        .args(["file", "cleanup-ds"])
        .assert()
        .success()
        .stdout(predicate::str::contains("not yet implemented"));

    forge()
        .args(["docker", "host"])
        .assert()
        .success()
        .stdout(predicate::str::contains("not yet implemented"));
}
