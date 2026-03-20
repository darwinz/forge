mod args;
mod output;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::Parser;

use forge_core::{
    bundles::{installer, ActionKind, BundleInventory, BundleRegistry, EnvironmentScan},
    commands::{aws, docker, file_ops, git, misc, notes, platform, shell_aliases, system, vercel},
    config,
    exec::{CommandRunner, DryRunRunner, RealRunner},
    logging, os,
    skills::{
        audit,
        discovery::{self, resolve_skills_dir},
        metadata::DiscoveredSkill,
        validation,
    },
};

use args::*;
use output::Printer;

fn main() -> Result<()> {
    let cli = Cli::parse();
    logging::init(cli.verbose);

    let config_path = cli.config.as_ref().map(|p| Path::new(p.as_str()));
    let config = config::load_config(config_path).context("failed to load config")?;

    let printer = Printer::new();

    let runner: Box<dyn CommandRunner> = if cli.dry_run {
        Box::new(DryRunRunner)
    } else {
        Box::new(RealRunner)
    };

    let platform = os::current_platform();

    match cli.command {
        Commands::Notes { command } => cmd_notes(&printer, command)?,
        Commands::System { command } => cmd_system(&printer, &*runner, &*platform, command)?,
        Commands::Bootstrap {
            list_bundles,
            status,
            format,
            scan,
            bundles,
            add,
            all,
            yes,
        } => {
            let bundle_dir = resolve_bundle_dir(&config);
            if list_bundles {
                cmd_bootstrap_list(&printer, &bundle_dir)?;
            } else if status {
                cmd_bootstrap_status(&printer, &bundle_dir, &*runner, &format)?;
            } else if scan {
                cmd_bootstrap_scan(&printer, &*runner)?;
            } else {
                cmd_bootstrap_install(
                    &printer,
                    &bundle_dir,
                    &*runner,
                    &config,
                    bundles.as_deref(),
                    add.as_deref(),
                    all,
                    yes,
                    cli.dry_run,
                )?;
            }
        }
        Commands::Skill { command } => {
            let user_dir = resolve_skills_dir(&config.skills.user_dir);
            let repo_dir = find_repo_skills_dir(&config.skills.repo_dir);
            cmd_skill(&printer, command, &user_dir, repo_dir.as_deref(), &config)?;
        }
        Commands::File { command } => cmd_file(&printer, &*runner, command, cli.dry_run)?,
        Commands::Docker { command } => {
            cmd_docker(&printer, &*runner, command, cli.dry_run)?;
        }
        Commands::Aws { command } => {
            cmd_aws(&printer, &*runner, command)?;
        }
        Commands::Platform { command } => {
            cmd_platform(&printer, &*runner, command)?;
        }
        Commands::Vercel { command } => {
            cmd_vercel(&printer, &*runner, command, cli.dry_run)?;
        }
        Commands::Git { command } => {
            let config_path = resolve_git_aliases_path(&config);
            cmd_git(&printer, &*runner, command, &config_path, cli.dry_run)?;
        }
        Commands::Misc { command } => cmd_misc(&printer, &*runner, command)?,
        Commands::Shell { command } => cmd_shell(&printer, command)?,
    }

    Ok(())
}

// --- Notes ---

fn cmd_notes(printer: &Printer, command: Option<NotesCommands>) -> Result<()> {
    match command {
        // `forge notes` with no args, `forge notes list`
        None | Some(NotesCommands::List) => {
            print_notes_list(printer);
        }
        // `forge notes <topic>`
        Some(NotesCommands::Topic(args)) => {
            if let Some(topic) = args.first() {
                let content = notes::get_notes(topic)?;
                printer.newline();
                printer.heading(&format!("Notes: {topic}"));
                printer.newline();
                printer.print(content);
                printer.newline();
            } else {
                print_notes_list(printer);
            }
        }
    }
    Ok(())
}

fn print_notes_list(printer: &Printer) {
    printer.newline();
    printer.heading("Available notes topics:");
    printer.newline();
    let topics: Vec<(&str, &str)> = notes::list_topics();
    printer.table(
        &topics.iter().map(|(k, v)| (*k, *v)).collect::<Vec<_>>(),
        16,
    );
    printer.newline();
    printer.dim("Usage: forge notes <topic>");
    printer.newline();
}

// --- System ---

fn cmd_system(
    printer: &Printer,
    runner: &dyn CommandRunner,
    platform: &dyn os::OsPlatform,
    command: SystemCommands,
) -> Result<()> {
    let output = match command {
        SystemCommands::CpuHogs => system::cpu_hogs(runner)?,
        SystemCommands::MemHogs => system::mem_hogs_top(runner)?,
        SystemCommands::MemHogsPs => system::mem_hogs_ps(runner)?,
        SystemCommands::Top => system::top_snapshot(runner)?,
        SystemCommands::FindPid { name } => system::find_pid(runner, &name)?,
        SystemCommands::Ps => system::my_ps(runner)?,
        SystemCommands::NetCons => system::net_cons(runner)?,
        SystemCommands::Port { port } => system::used_port(runner, port)?,
        SystemCommands::IpInfo { interface } => system::ip_info(platform, runner, &interface)?,
        SystemCommands::Hardware => system::hardware(platform, runner)?,
        SystemCommands::Ping6 => system::ping_ipv6(runner)?,
    };

    if !output.is_empty() {
        printer.print(&output);
    }

    Ok(())
}

// --- Bootstrap ---

fn resolve_bundle_dir(config: &config::ForgeConfig) -> PathBuf {
    // Try relative to the binary/project first, then absolute
    let dir = PathBuf::from(&config.bootstrap.bundle_dir);
    if dir.exists() {
        return dir;
    }

    // Try relative to the executable
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            let candidate = exe_dir.join("../../config/bundles");
            if candidate.exists() {
                return candidate;
            }
        }
    }

    // Try project root (for development)
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let project_root = Path::new(manifest_dir).parent().and_then(|p| p.parent());
    if let Some(root) = project_root {
        let candidate = root.join("config/bundles");
        if candidate.exists() {
            return candidate;
        }
    }

    dir
}

fn cmd_bootstrap_list(printer: &Printer, bundle_dir: &Path) -> Result<()> {
    let registry = BundleRegistry::load(bundle_dir).context("failed to load bundle manifests")?;

    printer.newline();
    printer.heading("Available bundles:");
    printer.newline();

    for (tier, bundles) in registry.list_by_tier() {
        let tier_label = match tier {
            "core" => "Core (always installed)",
            "role" => "Role-specific (opt-in)",
            "experimental" => "Experimental (opt-in)",
            "ai-tools" => "AI Tools (inventory & recommend)",
            _ => tier,
        };
        printer.bold(&format!("  {tier_label}:"));
        for b in bundles {
            let required = if b.bundle.required { " [required]" } else { "" };
            println!(
                "    {:<16} {}{}",
                b.bundle.name, b.bundle.description, required
            );
        }
        printer.newline();
    }

    Ok(())
}

fn cmd_bootstrap_status(
    printer: &Printer,
    bundle_dir: &Path,
    runner: &dyn CommandRunner,
    format: &str,
) -> Result<()> {
    let registry = BundleRegistry::load(bundle_dir).context("failed to load bundle manifests")?;

    let mut all_reports = Vec::new();
    for manifest in registry.all() {
        all_reports.push(BundleInventory::check(manifest, runner)?);
    }

    if format == "json" {
        print_status_json(&all_reports);
        return Ok(());
    }

    printer.newline();
    if runner.is_dry_run() {
        printer.heading("Package inventory report (dry-run — no queries executed):");
    } else {
        printer.heading("Package inventory report:");
    }
    printer.newline();

    for report in &all_reports {
        printer.bold(&format!(
            "  {} ({}/{} — {})",
            report.bundle_name,
            report.installed_count(),
            report.total_count(),
            report.summary_label(),
        ));

        for pkg in &report.packages {
            let ver = pkg.version.as_deref().unwrap_or("");
            let label = if ver.is_empty() {
                format!("{} ({})", pkg.name, pkg.source_label())
            } else {
                format!("{} {} ({})", pkg.name, ver, pkg.source_label())
            };
            printer.package_state(&label, pkg.state.label());
        }
        printer.newline();
    }

    Ok(())
}

fn print_status_json(reports: &[forge_core::bundles::inventory::BundleReport]) {
    // Manual JSON output to avoid serde_json dependency
    println!("{{");
    println!("  \"bundles\": [");
    for (i, report) in reports.iter().enumerate() {
        println!("    {{");
        println!("      \"name\": \"{}\",", report.bundle_name);
        println!("      \"tier\": \"{}\",", report.tier);
        println!("      \"installed\": {},", report.installed_count());
        println!("      \"total\": {},", report.total_count());
        println!("      \"summary\": \"{}\",", report.summary_label());
        println!("      \"packages\": [");
        for (j, pkg) in report.packages.iter().enumerate() {
            let ver = pkg.version.as_deref().unwrap_or("");
            let comma = if j + 1 < report.packages.len() {
                ","
            } else {
                ""
            };
            println!(
                "        {{\"name\": \"{}\", \"source\": \"{}\", \"version\": \"{}\", \"state\": \"{}\"}}{}",
                pkg.name, pkg.source_label(), ver, pkg.state.label(), comma
            );
        }
        println!("      ]");
        let comma = if i + 1 < reports.len() { "," } else { "" };
        println!("    }}{comma}");
    }
    println!("  ]");
    println!("}}");
}

fn cmd_bootstrap_scan(printer: &Printer, runner: &dyn CommandRunner) -> Result<()> {
    printer.newline();
    printer.heading("Environment scan:");
    printer.newline();

    let scan = EnvironmentScan::run(runner);

    // -- Package managers --
    if !scan.package_manager.is_empty() {
        printer.bold("  Package-manager-installed tools:");
        // Group by source kind
        let mut by_source: std::collections::BTreeMap<
            String,
            Vec<&forge_core::bundles::sources::DetectedPackage>,
        > = std::collections::BTreeMap::new();
        for pkg in &scan.package_manager {
            by_source
                .entry(pkg.source.to_string())
                .or_default()
                .push(pkg);
        }
        for (source, pkgs) in &by_source {
            printer.dim(&format!("    [{source}]"));
            for pkg in pkgs {
                let ver = pkg.version.as_deref().unwrap_or("");
                if ver.is_empty() {
                    println!("      {}", pkg.name);
                } else {
                    println!("      {} {ver}", pkg.name);
                }
            }
        }
        printer.newline();
    }

    // -- Version managers --
    if !scan.version_manager.is_empty() {
        printer.bold("  Version-manager toolchains:");
        for pkg in &scan.version_manager {
            let ver = pkg.version.as_deref().unwrap_or("(no version)");
            println!("    {} {} [{}]", pkg.name, ver, pkg.source);
        }
        printer.newline();
    }

    // -- Standalone binaries --
    if !scan.standalone_binaries.is_empty() {
        printer.bold("  Standalone binaries in well-known directories:");
        for (dir, names) in &scan.standalone_binaries {
            printer.dim(&format!("    {} ({} binaries)", dir.display(), names.len()));
            // Show first 15 names, truncate if more
            let show = if names.len() > 15 { 15 } else { names.len() };
            let mut display_names: Vec<&str> =
                names.iter().take(show).map(|s| s.as_str()).collect();
            display_names.sort();
            println!("      {}", display_names.join(", "));
            if names.len() > show {
                printer.dim(&format!("      ... and {} more", names.len() - show));
            }
        }
        printer.newline();
    }

    // -- Unavailable managers --
    if !scan.unavailable_managers.is_empty() {
        printer.dim(&format!(
            "  Managers not found: {}",
            scan.unavailable_managers.join(", ")
        ));
        printer.newline();
    }

    if scan.package_manager.is_empty()
        && scan.version_manager.is_empty()
        && scan.standalone_binaries.is_empty()
    {
        printer.dim("  No additional tools detected.");
        printer.newline();
    }

    Ok(())
}

// --- Bootstrap install ---

#[allow(clippy::too_many_arguments)]
fn cmd_bootstrap_install(
    printer: &Printer,
    bundle_dir: &Path,
    runner: &dyn CommandRunner,
    config: &config::ForgeConfig,
    bundles_arg: Option<&str>,
    add_arg: Option<&str>,
    all: bool,
    yes: bool,
    dry_run: bool,
) -> Result<()> {
    let registry = BundleRegistry::load(bundle_dir).context("failed to load bundle manifests")?;

    // Resolve which bundles to install
    let bundle_names: Vec<String> = if all {
        registry
            .all()
            .iter()
            .map(|b| b.bundle.name.clone())
            .collect()
    } else if let Some(names) = bundles_arg {
        names.split(',').map(|s| s.trim().to_string()).collect()
    } else if let Some(name) = add_arg {
        vec![name.to_string()]
    } else {
        // Load default profile
        let profile_path = resolve_profile_path(config);
        installer::load_default_profile(&profile_path).unwrap_or_else(|_| vec!["core".to_string()])
    };

    // Validate bundle names
    let mut manifests = Vec::new();
    for name in &bundle_names {
        match registry.find(name) {
            Some(m) => manifests.push(m),
            None => {
                printer.error(&format!("Unknown bundle: {name}"));
                printer.dim("Run `forge bootstrap --list-bundles` to see available bundles.");
                return Ok(());
            }
        }
    }

    // Build install plan
    let plan = installer::plan_install(&manifests, runner)?;

    printer.newline();
    printer.heading("Bootstrap plan:");
    printer.newline();

    // Show what's already installed
    let already = plan.already_installed();
    if !already.is_empty() {
        printer.dim(&format!("  Already installed: {} packages", already.len()));
    }

    // Show what will be installed
    let to_install = plan.to_install();
    if !to_install.is_empty() {
        printer.bold(&format!("  Will install: {} packages", to_install.len()));
        for action in &to_install {
            println!("    [{}] {}", action.source, action.package);
        }
    }

    // Show skipped
    let skipped = plan.skipped();
    if !skipped.is_empty() {
        printer.newline();
        printer.dim(&format!(
            "  Skipped: {} packages (unsupported source for auto-install)",
            skipped.len()
        ));
        for action in &skipped {
            if let ActionKind::Skipped { reason } = &action.kind {
                println!("    {} — {reason}", action.package);
            }
        }
    }

    // Show manual entries
    let manual = plan.manual();
    if !manual.is_empty() {
        printer.newline();
        printer.bold("  Manual install required:");
        for action in &manual {
            if let ActionKind::Manual {
                instructions,
                notes,
                detected,
            } = &action.kind
            {
                let status = if *detected {
                    "detected"
                } else {
                    "not detected"
                };
                println!("    {} [{status}]", action.package);
                println!("      Install: {instructions}");
                if let Some(n) = notes {
                    println!("      Note: {n}");
                }
            }
        }
    }

    printer.newline();

    // Nothing to do?
    if !plan.has_work() {
        printer.success("Nothing to install — all supported packages are present.");
        return Ok(());
    }

    // Dry-run stops here
    if dry_run {
        printer.dim("[dry-run] No packages will be installed.");
        return Ok(());
    }

    // Confirmation
    if !yes {
        printer.print(&format!(
            "Proceed with installing {} packages? [y/N] ",
            to_install.len()
        ));
        let mut input = String::new();
        std::io::stdin()
            .read_line(&mut input)
            .context("failed to read stdin")?;
        let answer = input.trim().to_lowercase();
        if answer != "y" && answer != "yes" {
            printer.dim("Aborted.");
            return Ok(());
        }
    }

    // Execute
    printer.newline();
    printer.heading("Installing...");
    printer.newline();

    let results = installer::execute_plan(&plan, runner);

    let succeeded: Vec<_> = results.iter().filter(|r| r.success).collect();
    let failed: Vec<_> = results.iter().filter(|r| !r.success).collect();

    if !succeeded.is_empty() {
        printer.success(&format!("  Installed {} packages:", succeeded.len()));
        for r in &succeeded {
            println!("    [{}] {}", r.source, r.package);
        }
    }

    if !failed.is_empty() {
        printer.newline();
        printer.error(&format!("  Failed {} packages:", failed.len()));
        for r in &failed {
            println!("    [{}] {} — {}", r.source, r.package, r.message);
        }
    }

    // Show PATH hints for sources that install to non-standard locations
    let hints = installer::path_hints_for_results(&results);
    if !hints.is_empty() {
        printer.newline();
        printer.bold("  PATH notes:");
        for hint in &hints {
            println!("    {hint}");
        }
    }

    printer.newline();
    Ok(())
}

fn resolve_profile_path(config: &config::ForgeConfig) -> PathBuf {
    let path = PathBuf::from(&config.bootstrap.default_profile);
    if path.exists() {
        return path;
    }
    // Try relative to project root
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let project_root = Path::new(manifest_dir).parent().and_then(|p| p.parent());
    if let Some(root) = project_root {
        let candidate = root.join(&config.bootstrap.default_profile);
        if candidate.exists() {
            return candidate;
        }
    }
    path
}

// --- Skills ---

fn find_repo_skills_dir(repo_dir_config: &str) -> Option<PathBuf> {
    // Walk up from CWD to find git root, then look for .forge/skills
    let cwd = std::env::current_dir().ok()?;
    let mut dir = cwd.as_path();
    loop {
        if dir.join(".git").exists() {
            let candidate = dir.join(repo_dir_config);
            if candidate.exists() {
                return Some(candidate);
            }
            return None;
        }
        dir = dir.parent()?;
    }
}

fn cmd_skill(
    printer: &Printer,
    command: SkillCommands,
    user_dir: &Path,
    repo_dir: Option<&Path>,
    config: &config::ForgeConfig,
) -> Result<()> {
    match command {
        SkillCommands::List { tag } => {
            let skills = discovery::discover_skills(user_dir, repo_dir)?;
            printer.newline();
            printer.heading("Discovered skills:");
            printer.newline();

            let filtered: Vec<&DiscoveredSkill> = if let Some(ref tag_filter) = tag {
                skills
                    .iter()
                    .filter(|s| {
                        s.manifest
                            .skill
                            .metadata
                            .as_ref()
                            .map(|m| m.tags.iter().any(|t| t == tag_filter))
                            .unwrap_or(false)
                    })
                    .collect()
            } else {
                skills.iter().collect()
            };

            if filtered.is_empty() {
                printer.dim("  No skills found.");
                printer.dim(&format!("  User dir: {}", user_dir.display()));
                if let Some(rd) = repo_dir {
                    printer.dim(&format!("  Repo dir: {}", rd.display()));
                }
            } else {
                for skill in &filtered {
                    println!(
                        "  {:<24} {} [{}]",
                        skill.manifest.skill.name, skill.manifest.skill.description, skill.source,
                    );
                }
            }
            printer.newline();
        }

        SkillCommands::Info { name } => {
            let skills = discovery::discover_skills(user_dir, repo_dir)?;
            let skill = skills.iter().find(|s| s.manifest.skill.name == name);

            match skill {
                Some(s) => {
                    printer.newline();
                    printer.heading(&format!("Skill: {}", s.manifest.skill.name));
                    printer.newline();
                    println!("  Version:     {}", s.manifest.skill.version);
                    println!("  Description: {}", s.manifest.skill.description);
                    if let Some(ref author) = s.manifest.skill.author {
                        println!("  Author:      {author}");
                    }
                    println!("  Source:      {}", s.source);
                    println!("  Path:        {}", s.path.display());

                    if let Some(ref meta) = s.manifest.skill.metadata {
                        if !meta.tags.is_empty() {
                            println!("  Tags:        {}", meta.tags.join(", "));
                        }
                        if let Some(ref cat) = meta.category {
                            println!("  Category:    {cat}");
                        }
                    }

                    if let Some(ref perms) = s.manifest.skill.permissions {
                        printer.newline();
                        printer.bold("  Permissions:");
                        println!("    read_files:       {}", perms.read_files);
                        println!("    write_files:      {}", perms.write_files);
                        println!("    network:          {}", perms.network);
                        println!("    execute_commands: {}", perms.execute_commands);
                        if let Some(max) = perms.max_file_size_bytes {
                            println!("    max_file_size:    {} bytes", max);
                        }
                        if let Some(timeout) = perms.timeout_seconds {
                            println!("    timeout:          {}s", timeout);
                        }
                    }
                    printer.newline();
                }
                None => {
                    printer.error(&format!("Skill not found: {name}"));
                    return Ok(());
                }
            }
        }

        SkillCommands::Validate { name, all } => {
            let skills = discovery::discover_skills(user_dir, repo_dir)?;

            let to_validate: Vec<&DiscoveredSkill> = if all {
                skills.iter().collect()
            } else if let Some(ref n) = name {
                skills
                    .iter()
                    .filter(|s| s.manifest.skill.name == *n)
                    .collect()
            } else {
                printer.error("Specify a skill name or --all");
                return Ok(());
            };

            if to_validate.is_empty() {
                printer.dim("No skills to validate.");
                return Ok(());
            }

            printer.newline();
            printer.heading("Skill validation:");
            printer.newline();

            for skill in to_validate {
                let result = validation::validate_skill(skill);
                if result.is_valid() {
                    printer.success(&format!("  {} — valid", result.skill_name));
                } else {
                    printer.error(&format!("  {} — invalid", result.skill_name));
                }
                for err in &result.errors {
                    println!("    error: {err}");
                }
                for warn in &result.warnings {
                    println!("    warning: {warn}");
                }
            }
            printer.newline();
        }

        SkillCommands::Link { path } => {
            let source = PathBuf::from(&path);
            if !source.join("skill.toml").exists() {
                printer.error(&format!("No skill.toml found in {path}"));
                return Ok(());
            }

            let skill_name = source
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "unknown".to_string());

            let link_target = user_dir.join(&skill_name);
            printer.dim(&format!(
                "Would symlink {} -> {}",
                link_target.display(),
                source.display()
            ));
            printer.dim("Skill linking requires creating directories (Phase 2+).");
        }

        SkillCommands::Audit { skill: _ } => {
            let audit_path = resolve_skills_dir(&config.skills.audit_log);
            let records = audit::read_audit_log(&audit_path);

            if records.is_empty() {
                printer.dim("No audit records found.");
                printer.dim("Audit logging begins when skill execution is implemented (v2+).");
            } else {
                for record in &records {
                    println!(
                        "  {} {} v{} — {} ({}ms)",
                        record.timestamp,
                        record.skill_name,
                        record.skill_version,
                        record.action,
                        record.duration_ms,
                    );
                }
            }
        }
    }

    Ok(())
}

// --- File ---

fn cmd_file(
    printer: &Printer,
    runner: &dyn CommandRunner,
    command: FileCommands,
    dry_run: bool,
) -> Result<()> {
    match command {
        // --- Read-only ---
        FileCommands::Find { pattern } => {
            let output = file_ops::find_file(runner, &pattern)?;
            if !output.is_empty() {
                printer.print(&output);
            }
        }
        FileCommands::Search { pattern } => {
            let output = file_ops::search(runner, &pattern)?;
            if !output.is_empty() {
                printer.print(&output);
            }
        }
        FileCommands::Spotlight { query } => {
            let output = file_ops::spotlight(runner, &query)?;
            if !output.is_empty() {
                printer.print(&output);
            }
        }

        // --- Mutation ---
        FileCommands::Extract { file } => {
            let output = file_ops::extract(runner, &file)?;
            if !output.is_empty() {
                printer.success(&output);
            }
        }
        FileCommands::Trash { file } => {
            let output = file_ops::trash(runner, &file)?;
            if !output.is_empty() {
                printer.success(&output);
            }
        }
        FileCommands::CleanupDs { yes } => {
            let files = file_ops::find_ds_store(runner)?;

            if files.is_empty() {
                printer.dim("No .DS_Store files found.");
                return Ok(());
            }

            printer.newline();
            printer.heading(&format!("Found {} .DS_Store file(s):", files.len()));
            printer.newline();
            for f in &files {
                println!("  {f}");
            }
            printer.newline();

            if dry_run {
                printer.dim("[dry-run] No files will be removed.");
                return Ok(());
            }

            if !yes
                && !confirm_destructive(
                    printer,
                    &format!("Remove {} .DS_Store file(s)?", files.len()),
                )?
            {
                printer.dim("Aborted.");
                return Ok(());
            }

            let result = file_ops::remove_ds_store(runner, &files)?;
            printer.success(&result);
        }
    }
    Ok(())
}

// --- Misc ---

fn cmd_misc(printer: &Printer, runner: &dyn CommandRunner, command: MiscCommands) -> Result<()> {
    match command {
        MiscCommands::Weather { location } => {
            let output = misc::weather(runner, location.as_deref())?;
            if !output.is_empty() {
                printer.print(&output);
            }
        }
        MiscCommands::Define { word } => {
            let output = misc::define(runner, &word)?;
            if !output.is_empty() {
                printer.print(&output);
            }
        }
    }
    Ok(())
}

// --- Shell ---

fn cmd_shell(printer: &Printer, command: ShellCommands) -> Result<()> {
    match command {
        ShellCommands::GenerateAliases => {
            // Pure stdout output — no file writes
            let output = shell_aliases::generate_aliases();
            printer.print(&output);
        }
    }
    Ok(())
}

// --- Docker ---

fn cmd_docker(
    printer: &Printer,
    runner: &dyn CommandRunner,
    command: DockerCommands,
    dry_run: bool,
) -> Result<()> {
    match command {
        DockerCommands::Host => {
            printer.print(&docker::docker_host());
        }
        DockerCommands::Images => {
            let output = docker::docker_images(runner)?;
            if !output.is_empty() {
                printer.print(&output);
            }
        }
        DockerCommands::Ps => {
            let output = docker::docker_ps(runner)?;
            if !output.is_empty() {
                printer.print(&output);
            }
        }
        DockerCommands::RmImages { yes } => {
            if !dry_run && !yes && !confirm_destructive(printer, "Remove ALL docker images?")? {
                printer.dim("Aborted.");
                return Ok(());
            }
            let output = docker::docker_rm_images(runner)?;
            if !output.is_empty() {
                printer.print(&output);
            }
        }
        DockerCommands::RmContainers { yes } => {
            if !dry_run && !yes && !confirm_destructive(printer, "Remove ALL docker containers?")? {
                printer.dim("Aborted.");
                return Ok(());
            }
            let output = docker::docker_rm_containers(runner)?;
            if !output.is_empty() {
                printer.print(&output);
            }
        }
        DockerCommands::RmByFilter { filter, yes } => {
            if !dry_run
                && !yes
                && !confirm_destructive(
                    printer,
                    &format!("Remove containers matching filter '{filter}'?"),
                )?
            {
                printer.dim("Aborted.");
                return Ok(());
            }
            let output = docker::docker_rm_by_filter(runner, &filter)?;
            if !output.is_empty() {
                printer.print(&output);
            }
        }
    }
    Ok(())
}

/// Prompt user for confirmation before a destructive operation.
fn confirm_destructive(_printer: &Printer, message: &str) -> Result<bool> {
    eprint!("{message} [y/N] ");
    let mut input = String::new();
    std::io::stdin()
        .read_line(&mut input)
        .context("failed to read stdin")?;
    let answer = input.trim().to_lowercase();
    Ok(answer == "y" || answer == "yes")
}

// --- AWS ---

fn cmd_aws(printer: &Printer, runner: &dyn CommandRunner, command: AwsCommands) -> Result<()> {
    match command {
        AwsCommands::Instances {
            profile,
            query,
            region,
        } => {
            let instances = aws::list_instances(
                runner,
                profile.as_deref(),
                region.as_deref(),
                query.as_deref(),
            )?;

            printer.newline();
            if let Some(ref q) = query {
                printer.heading(&format!("EC2 instances (filter: {q}):"));
            } else {
                printer.heading("EC2 instances:");
            }
            printer.newline();
            printer.print(&aws::format_instances(&instances));
            printer.newline();
            printer.dim(&format!("  {} instance(s) found", instances.len()));
            printer.newline();
        }

        AwsCommands::Ssh {
            profile,
            query,
            region,
            key,
            user,
        } => {
            let instances = aws::list_instances(
                runner,
                profile.as_deref(),
                region.as_deref(),
                query.as_deref(),
            )?;

            if runner.is_dry_run() {
                // In dry-run, just show what would happen
                printer.dim("[dry-run] Would resolve instance and SSH.");
                return Ok(());
            }

            if instances.is_empty() {
                printer.error("No instances found matching your query.");
                printer.dim("Try `forge aws instances` to see available instances.");
                return Ok(());
            }

            // If multiple instances, ask the user to narrow their query
            let target = if instances.len() == 1 {
                &instances[0]
            } else {
                printer.newline();
                printer.heading("Multiple instances found:");
                printer.newline();
                for (i, inst) in instances.iter().enumerate() {
                    let ip = inst.connect_ip().unwrap_or("-");
                    println!(
                        "  [{}] {} — {} ({})",
                        i + 1,
                        inst.name,
                        ip,
                        inst.instance_id
                    );
                }
                printer.newline();
                printer.print("Select instance [1]: ");
                let mut input = String::new();
                std::io::stdin()
                    .read_line(&mut input)
                    .context("failed to read stdin")?;
                let choice: usize = input.trim().parse::<usize>().unwrap_or(1).saturating_sub(1);
                if choice >= instances.len() {
                    printer.error("Invalid selection.");
                    return Ok(());
                }
                &instances[choice]
            };

            let ip = match target.connect_ip() {
                Some(ip) => ip,
                None => {
                    printer.error(&format!(
                        "Instance {} ({}) has no IP address.",
                        target.name, target.instance_id
                    ));
                    return Ok(());
                }
            };

            printer.dim(&format!(
                "Connecting to {} ({}) at {ip} as {user}...",
                target.name, target.instance_id
            ));

            let (prog, args) = aws::build_ssh_command(ip, &user, key.as_deref());
            printer.dim(&format!("  {} {}", prog, args.join(" ")));
            printer.newline();

            aws::exec_ssh(runner, ip, &user, key.as_deref())?;
        }

        AwsCommands::Connect {
            profile,
            lb_name,
            region,
        } => {
            let (lb_display, instances) = aws::list_lb_instances(
                runner,
                profile.as_deref(),
                region.as_deref(),
                lb_name.as_deref(),
            )?;

            printer.newline();
            printer.heading(&format!("Instances behind LB: {lb_display}"));
            printer.newline();

            if instances.is_empty() {
                printer.dim("  No instances found behind this load balancer.");
            } else {
                printer.print(&aws::format_instances(&instances));
                printer.newline();
                printer.dim(&format!("  {} instance(s)", instances.len()));
                printer
                    .dim("  Use `forge aws ssh --query <name>` to connect to a specific instance.");
            }
            printer.newline();
        }
    }
    Ok(())
}

// --- Vercel ---

fn cmd_vercel(
    printer: &Printer,
    runner: &dyn CommandRunner,
    command: VercelCommands,
    dry_run: bool,
) -> Result<()> {
    match command {
        VercelCommands::Doctor => {
            let result = vercel::doctor(runner);

            printer.newline();
            printer.heading("Vercel doctor:");
            printer.newline();

            // CLI
            if result.cli_installed {
                let ver = result.cli_version.as_deref().unwrap_or("unknown");
                printer.package_state(&format!("CLI: {ver}"), "installed");
            } else {
                printer.package_state("CLI: not installed", "missing");
            }

            // Auth
            if result.authenticated {
                let user = result.auth_user.as_deref().unwrap_or("authenticated");
                printer.package_state(&format!("Auth: {user}"), "installed");
            } else if result.cli_installed {
                printer.package_state("Auth: not logged in", "missing");
            }

            // Project
            if result.project_linked {
                let name = result.project_name.as_deref().unwrap_or("linked");
                printer.package_state(&format!("Project: {name}"), "installed");
            } else if result.cli_installed {
                printer.package_state("Project: not linked", "missing");
            }

            // Framework
            if let Some(ref fw) = result.framework {
                printer.dim(&format!("    Framework: {fw}"));
            }

            // Issues
            if !result.issues.is_empty() {
                printer.newline();
                printer.bold("  Issues:");
                for issue in &result.issues {
                    println!("    {issue}");
                }
            }

            // Suggestions
            if !result.suggestions.is_empty() {
                printer.newline();
                printer.bold("  Suggestions:");
                for suggestion in &result.suggestions {
                    println!("    {suggestion}");
                }
            }

            printer.newline();
        }

        VercelCommands::EnvDiff => {
            let diff = vercel::env_diff(runner)?;

            printer.newline();
            printer.heading("Vercel env diff:");
            printer.newline();
            printer.dim(&format!("  Local source: {}", diff.local_source));

            if !diff.remote_available {
                printer.dim("  Remote: not available (not linked or not logged in)");
                if !diff.local_only.is_empty() {
                    printer.newline();
                    printer.bold(&format!("  Local env vars ({}):", diff.local_only.len()));
                    for key in &diff.local_only {
                        println!("    {key}");
                    }
                }
                printer.newline();
                printer.dim("  Run `vercel link` and `vercel login` to enable remote comparison.");
                printer.newline();
                return Ok(());
            }

            // Full diff
            if !diff.both.is_empty() {
                printer.newline();
                printer.bold(&format!(
                    "  In both local and remote ({}):",
                    diff.both.len()
                ));
                for key in &diff.both {
                    println!("    {key}");
                }
            }

            if !diff.local_only.is_empty() {
                printer.newline();
                printer.bold(&format!(
                    "  Local only — not in Vercel ({}):",
                    diff.local_only.len()
                ));
                for key in &diff.local_only {
                    println!("    + {key}");
                }
            }

            if !diff.remote_only.is_empty() {
                printer.newline();
                printer.bold(&format!(
                    "  Remote only — not in local .env ({}):",
                    diff.remote_only.len()
                ));
                for key in &diff.remote_only {
                    println!("    - {key}");
                }
            }

            if diff.local_only.is_empty() && diff.remote_only.is_empty() {
                printer.newline();
                printer.success("  All env var keys match between local and remote.");
            }

            printer.newline();
        }

        VercelCommands::Deploy { prod, yes } => {
            let target = if prod { "production" } else { "preview" };

            printer.newline();
            printer.heading(&format!("Vercel deploy ({target}):"));
            printer.newline();

            // Pre-flight: check project is linked
            if !dry_run && !std::path::Path::new(".vercel/project.json").exists() {
                printer.error("Not linked to a Vercel project.");
                printer.dim("  Run: vercel link");
                printer.newline();
                return Ok(());
            }

            if dry_run {
                let args = vercel::deploy_args(prod);
                printer.dim(&format!(
                    "[dry-run] Would execute: vercel {}",
                    args.join(" ")
                ));
                printer.newline();
                return Ok(());
            }

            if !yes && !confirm_destructive(printer, &format!("Deploy to {target}?"))? {
                printer.dim("Aborted.");
                printer.newline();
                return Ok(());
            }

            printer.dim(&format!("  Deploying to {target}..."));
            printer.newline();

            match vercel::deploy(runner, prod) {
                Ok(url) => {
                    printer.success(&format!("  Deployed: {url}"));
                }
                Err(e) => {
                    printer.error(&format!("  Deploy failed: {e}"));
                }
            }
            printer.newline();
        }
    }
    Ok(())
}

// --- Platform ---

fn cmd_platform(
    printer: &Printer,
    runner: &dyn CommandRunner,
    command: PlatformCommands,
) -> Result<()> {
    match command {
        PlatformCommands::Status => {
            let statuses = platform::check_status(runner);
            printer.newline();
            printer.heading("Platform CLI status:");
            printer.newline();

            for s in &statuses {
                let marker = if s.installed { "installed" } else { "missing" };
                let version = s.version.as_deref().unwrap_or("");
                if s.installed {
                    printer.package_state(&format!("{:<12} {}", s.name, version), marker);
                } else {
                    printer.package_state(&s.name, marker);
                }
            }
            printer.newline();

            let installed_count = statuses.iter().filter(|s| s.installed).count();
            printer.dim(&format!(
                "  {}/{} platform CLIs installed",
                installed_count,
                statuses.len()
            ));
            printer.dim("  Install with: forge bootstrap --add platforms");
            printer.newline();
        }

        PlatformCommands::Doctor => {
            let results = platform::doctor(runner);
            printer.newline();
            printer.heading("Platform doctor:");
            printer.newline();

            for r in &results {
                if !r.installed {
                    printer.package_state(&format!("{}: not installed", r.name), "missing");
                    continue;
                }

                let version = r.version.as_deref().unwrap_or("unknown version");
                printer.bold(&format!("  {} ({})", r.name, version));

                // Auth
                match r.authenticated {
                    Some(true) => {
                        let detail = r.auth_detail.as_deref().unwrap_or("yes");
                        printer.package_state(&format!("  Auth: {detail}"), "installed");
                    }
                    Some(false) => {
                        let detail = r.auth_detail.as_deref().unwrap_or("not logged in");
                        printer.package_state(&format!("  Auth: {detail}"), "missing");
                    }
                    None => {
                        let detail = r.auth_detail.as_deref().unwrap_or("not checked");
                        printer.dim(&format!("    Auth: {detail}"));
                    }
                }

                // Project link
                match r.project_linked {
                    Some(true) => {
                        let detail = r.link_detail.as_deref().unwrap_or("linked");
                        printer.package_state(&format!("  Project: {detail}"), "installed");
                    }
                    Some(false) => {
                        printer.package_state("  Project: not linked", "missing");
                    }
                    None => {}
                }
            }
            printer.newline();
        }
    }
    Ok(())
}

// --- Git ---

fn resolve_git_aliases_path(config: &config::ForgeConfig) -> PathBuf {
    // Try config/git-aliases.toml relative to the project
    let default = "config/git-aliases.toml";
    let path = PathBuf::from(default);
    if path.exists() {
        return path;
    }

    // Try relative to the executable's project root
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let project_root = Path::new(manifest_dir).parent().and_then(|p| p.parent());
    if let Some(root) = project_root {
        let candidate = root.join(default);
        if candidate.exists() {
            return candidate;
        }
    }

    // Fallback: check if user set a custom path in config
    let _ = config; // reserved for future config.git.aliases_file
    path
}

fn cmd_git(
    printer: &Printer,
    runner: &dyn CommandRunner,
    command: GitCommands,
    aliases_path: &Path,
    dry_run: bool,
) -> Result<()> {
    match command {
        GitCommands::ListAliases => {
            let aliases =
                git::load_aliases(aliases_path).context("failed to load git aliases config")?;
            printer.newline();
            printer.heading("Configured git aliases:");
            printer.newline();
            printer.print(&git::format_aliases(&aliases));
            printer.newline();
            printer.dim(&format!("  Source: {}", aliases_path.display()));
            printer.newline();
        }
        GitCommands::SetupAliases => {
            let aliases =
                git::load_aliases(aliases_path).context("failed to load git aliases config")?;

            if aliases.is_empty() {
                printer.dim("No aliases found in config.");
                return Ok(());
            }

            // Show diff preview
            let diff = git::diff_aliases(runner, &aliases)?;
            let git::AliasDiff {
                to_add,
                to_change,
                unchanged,
            } = diff;

            printer.newline();
            printer.heading("Git alias setup preview:");
            printer.newline();

            if !to_add.is_empty() {
                printer.bold(&format!("  Will add {} alias(es):", to_add.len()));
                for alias in &to_add {
                    println!("    + git {} → {}", alias.name, alias.value);
                }
            }

            if !to_change.is_empty() {
                printer.newline();
                printer.bold(&format!("  Will update {} alias(es):", to_change.len()));
                for (alias, old_val) in &to_change {
                    println!("    ~ git {} : {} → {}", alias.name, old_val, alias.value);
                }
            }

            if !unchanged.is_empty() {
                printer.newline();
                printer.dim(&format!(
                    "  Already up-to-date: {} alias(es)",
                    unchanged.len()
                ));
            }

            printer.newline();

            if to_add.is_empty() && to_change.is_empty() {
                printer.success("All aliases are already up-to-date.");
                return Ok(());
            }

            // Dry-run stops here
            if dry_run {
                printer.dim("[dry-run] No aliases will be applied.");
                return Ok(());
            }

            // Collect aliases to apply
            let mut to_apply: Vec<git::GitAlias> = to_add;
            to_apply.extend(to_change.into_iter().map(|(alias, _)| alias));

            let results = git::apply_aliases(runner, &to_apply)?;

            let succeeded: Vec<_> = results.iter().filter(|r| r.success).collect();
            let failed: Vec<_> = results.iter().filter(|r| !r.success).collect();

            if !succeeded.is_empty() {
                printer.success(&format!("  Applied {} alias(es).", succeeded.len()));
            }
            if !failed.is_empty() {
                printer.error(&format!("  Failed {} alias(es):", failed.len()));
                for r in &failed {
                    println!("    {} — failed to set", r.name);
                }
            }
            printer.newline();
        }
    }
    Ok(())
}
