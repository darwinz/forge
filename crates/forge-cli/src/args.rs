use clap::{builder::styling, Parser, Subcommand};

fn styles() -> styling::Styles {
    styling::Styles::styled()
        .header(styling::AnsiColor::Cyan.on_default().bold())
        .usage(styling::AnsiColor::Cyan.on_default().bold())
        .literal(styling::AnsiColor::Green.on_default())
        .placeholder(styling::AnsiColor::Yellow.on_default())
}

#[derive(Parser)]
#[command(
    name = "forge",
    about = "Developer toolkit — bootstrap, manage, and extend your dev environment",
    version,
    long_about = None,
    styles = styles(),
    after_help = "Run `forge <command> --help` for details on a specific command.\n\
                  Run `forge notes` to browse built-in cheatsheets (38 topics).",
)]
pub struct Cli {
    /// Enable dry-run mode (no mutations, preview only)
    #[arg(long, global = true)]
    pub dry_run: bool,

    /// Increase verbosity (-v info, -vv debug, -vvv trace)
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    /// Path to config file
    #[arg(long, global = true)]
    pub config: Option<String>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Developer cheatsheets and quick references
    Notes {
        #[command(subcommand)]
        command: Option<NotesCommands>,
    },

    /// System monitoring and info
    #[command(arg_required_else_help = true)]
    System {
        #[command(subcommand)]
        command: SystemCommands,
    },

    /// File operations and search
    #[command(
        arg_required_else_help = true,
        after_help = "Read-only: find, search, spotlight\nMutation:  extract, trash, cleanup-ds (requires confirmation)"
    )]
    File {
        #[command(subcommand)]
        command: FileCommands,
    },

    /// Docker utilities
    #[command(arg_required_else_help = true)]
    Docker {
        #[command(subcommand)]
        command: DockerCommands,
    },

    /// AWS operations
    #[command(arg_required_else_help = true)]
    Aws {
        #[command(subcommand)]
        command: AwsCommands,
    },

    /// Git alias management
    #[command(arg_required_else_help = true)]
    Git {
        #[command(subcommand)]
        command: GitCommands,
    },

    /// Platform CLI diagnostics (Vercel, Supabase, Netlify, Render, Appwrite)
    #[command(arg_required_else_help = true)]
    Platform {
        #[command(subcommand)]
        command: PlatformCommands,
    },

    /// Vercel workflows — doctor, env-diff, deploy
    #[command(arg_required_else_help = true)]
    Vercel {
        #[command(subcommand)]
        command: VercelCommands,
    },

    /// Supabase workflows — doctor, migration status, local reset
    #[command(arg_required_else_help = true)]
    Supabase {
        #[command(subcommand)]
        command: SupabaseCommands,
    },

    /// Netlify workflows — doctor, env-diff
    #[command(arg_required_else_help = true)]
    Netlify {
        #[command(subcommand)]
        command: NetlifyCommands,
    },

    /// Miscellaneous utilities
    #[command(arg_required_else_help = true)]
    Misc {
        #[command(subcommand)]
        command: MiscCommands,
    },

    /// Environment bootstrap — install and manage tool bundles
    Bootstrap {
        /// List available bundles
        #[arg(long)]
        list_bundles: bool,

        /// Show package inventory / drift report
        #[arg(long)]
        status: bool,

        /// Output format for --status (text or json)
        #[arg(long, default_value = "text")]
        format: String,

        /// Scan environment for tools beyond bundles (pipx, uv, pnpm, bun, composer, asdf, mise, bin dirs)
        #[arg(long)]
        scan: bool,

        /// Bundles to install (comma-separated)
        #[arg(long)]
        bundles: Option<String>,

        /// Add a single bundle
        #[arg(long)]
        add: Option<String>,

        /// Install all bundles
        #[arg(long)]
        all: bool,

        /// Skip confirmation prompt (non-interactive)
        #[arg(long, short = 'y')]
        yes: bool,
    },

    /// AI skill management (v1: metadata only, no execution)
    #[command(arg_required_else_help = true)]
    Skill {
        #[command(subcommand)]
        command: SkillCommands,
    },

    /// Shell integration
    #[command(arg_required_else_help = true)]
    Shell {
        #[command(subcommand)]
        command: ShellCommands,
    },
}

/// Notes subcommands — topic names double as subcommands for discoverability
#[derive(Subcommand)]
pub enum NotesCommands {
    /// List all available notes topics
    List,
    /// Show a specific notes topic
    #[command(external_subcommand)]
    Topic(Vec<String>),
}

#[derive(Subcommand)]
pub enum SystemCommands {
    /// Top CPU-consuming processes
    CpuHogs,
    /// Top memory-consuming processes (via top)
    MemHogs,
    /// Top memory-consuming processes (via ps)
    MemHogsPs,
    /// CPU/memory snapshot via top (single sample)
    Top,
    /// Find process by name
    FindPid {
        /// Process name to search for
        name: String,
    },
    /// List your processes
    Ps,
    /// Open TCP/IP sockets
    NetCons,
    /// Process using a specific port
    Port {
        /// Port number
        port: u16,
    },
    /// Network interface info
    IpInfo {
        /// Interface (en0, en1)
        #[arg(default_value = "en0")]
        interface: String,
    },
    /// System hardware info
    Hardware,
    /// Ping IPv6 multicast
    Ping6,
}

#[derive(Subcommand)]
pub enum FileCommands {
    // --- Read-only ---
    /// Find file by name
    Find {
        /// Filename pattern
        pattern: String,
    },
    /// Spotlight search (macOS)
    Spotlight {
        /// Search query
        query: String,
    },
    /// Recursive grep search
    Search {
        /// Search pattern
        pattern: String,
    },

    // --- Mutation ---
    /// Extract an archive (tar, zip, gz, bz2, rar, 7z)
    Extract {
        /// Archive file path
        file: String,
    },
    /// Move file to trash (no permanent delete)
    Trash {
        /// File or directory to trash
        file: String,
    },
    /// Remove .DS_Store files recursively (requires confirmation)
    CleanupDs {
        /// Skip confirmation prompt
        #[arg(long, short = 'y')]
        yes: bool,
    },
}

#[derive(Subcommand)]
pub enum DockerCommands {
    /// Show DOCKER_HOST environment variable
    Host,
    /// List docker images
    Images,
    /// List running and stopped containers
    Ps,
    /// Remove all docker images (requires confirmation)
    RmImages {
        /// Skip confirmation prompt
        #[arg(long, short = 'y')]
        yes: bool,
    },
    /// Remove all docker containers (requires confirmation)
    RmContainers {
        /// Skip confirmation prompt
        #[arg(long, short = 'y')]
        yes: bool,
    },
    /// Remove containers matching a docker filter (requires confirmation)
    RmByFilter {
        /// Docker filter expression (e.g. "status=exited", "name=myapp")
        filter: String,
        /// Skip confirmation prompt
        #[arg(long, short = 'y')]
        yes: bool,
    },
}

#[derive(Subcommand)]
pub enum AwsCommands {
    /// List EC2 instances (uses `aws ec2 describe-instances`)
    Instances {
        /// AWS profile name (maps to --profile)
        #[arg(short, long)]
        profile: Option<String>,
        /// Filter instances by Name tag (substring match)
        #[arg(short, long)]
        query: Option<String>,
        /// AWS region override
        #[arg(short, long)]
        region: Option<String>,
    },
    /// SSH to an EC2 instance
    Ssh {
        /// AWS profile name
        #[arg(short, long)]
        profile: Option<String>,
        /// Filter instances by Name tag to find the target
        #[arg(short, long)]
        query: Option<String>,
        /// AWS region override
        #[arg(short, long)]
        region: Option<String>,
        /// Path to SSH private key file (e.g. ~/.ssh/mykey.pem)
        #[arg(short, long)]
        key: Option<String>,
        /// SSH user
        #[arg(short, long, default_value = "ubuntu")]
        user: String,
    },
    /// List instances behind a load balancer
    Connect {
        /// AWS profile name
        #[arg(short, long)]
        profile: Option<String>,
        /// Load balancer name (exact or substring match)
        #[arg(long)]
        lb_name: Option<String>,
        /// AWS region override
        #[arg(short, long)]
        region: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum PlatformCommands {
    /// Check platform CLI installation, auth, and project link status
    Doctor,
    /// Show which platform CLIs are installed and their versions
    Status,
}

#[derive(Subcommand)]
pub enum VercelCommands {
    /// Check Vercel CLI installation, auth, project link, and framework detection
    Doctor,
    /// Compare local .env files against Vercel environment variables
    EnvDiff,
    /// Show latest deployment status
    Status,
    /// Deploy to Vercel (requires confirmation)
    Deploy {
        /// Deploy to production (default is preview)
        #[arg(long)]
        prod: bool,
        /// Skip confirmation prompt
        #[arg(long, short = 'y')]
        yes: bool,
    },
}

#[derive(Subcommand)]
pub enum SupabaseCommands {
    /// Check Supabase CLI installation, Docker, project link, and local stack status
    Doctor,
    /// Show local migration files and remote migration status
    MigrationStatus,
    /// Show local Supabase service URLs and status
    Services,
    /// Reset local database to clean state (requires confirmation)
    Reset {
        /// Skip confirmation prompt
        #[arg(long, short = 'y')]
        yes: bool,
    },
}

#[derive(Subcommand)]
pub enum NetlifyCommands {
    /// Check Netlify CLI installation, auth, and site link status
    Doctor,
    /// Compare local .env files against Netlify environment variables
    EnvDiff,
}

#[derive(Subcommand)]
pub enum GitCommands {
    /// Apply git aliases from config
    SetupAliases,
    /// List configured git aliases
    ListAliases,
}

#[derive(Subcommand)]
pub enum MiscCommands {
    /// Get weather for a location
    Weather {
        /// Location (city name or zip code)
        location: Option<String>,
    },
    /// Look up a word definition
    Define {
        /// Word to define
        word: String,
    },
}

#[derive(Subcommand)]
pub enum SkillCommands {
    /// List discovered skills
    List {
        /// Filter by tag
        #[arg(long)]
        tag: Option<String>,
    },
    /// Show full skill metadata
    Info {
        /// Skill name
        name: String,
    },
    /// Validate skill manifest(s)
    Validate {
        /// Skill name (or --all)
        name: Option<String>,
        /// Validate all discovered skills
        #[arg(long)]
        all: bool,
    },
    /// Symlink a local skill for development
    Link {
        /// Path to skill directory
        path: String,
    },
    /// View audit log
    Audit {
        /// Filter by skill name
        #[arg(long)]
        skill: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum ShellCommands {
    /// Generate shell aliases file to stdout
    GenerateAliases,
}
