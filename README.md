# forge

A developer toolkit CLI for bootstrapping, managing, and extending your dev environment. Written in Rust, replacing the previous Bash-based `mycli`.

## Why forge exists

The original `mycli` was ~16 shell scripts concatenated into a single sourced file. It worked, but had real problems: `curl|bash` installers with no checksums, `StrictHostKeyChecking=no` on every SSH connection, DES3 encryption, plaintext VPN credentials, hardcoded users, and fragile `awk`-based profile mutation.

`forge` is a ground-up rewrite that keeps the useful parts (cheatsheets, system monitoring, package inventories, shell aliases) while dropping the dangerous ones. Security, maintainability, and an AI skills subsystem are first-class priorities — not a line-by-line port.

## Current status

forge is in active development. The read-only foundation is solid and usable today. Installation and mutation features are rolling out incrementally with safety controls.

| Area | Status |
|------|--------|
| CLI skeleton, help, subcommand tree | Done |
| Notes/cheatsheets (33 topics) | Done |
| System monitoring (read-only) | Done |
| File operations (find, search, spotlight, extract, trash, cleanup-ds) | Done |
| Misc (`weather`, `define`) | Done |
| Shell alias generation | Done |
| Bundle manifests and registry | Done |
| Package inventory/status/drift reporting | Done |
| Environment scan (pipx, uv, pnpm, bun, asdf, mise, bin dirs) | Done |
| Bootstrap install (brew, brew cask, npm, go, gem, uv tool) | Done |
| Skills subsystem (metadata, discovery, validation) | Done (no execution) |
| `--dry-run` across all commands | Done |
| Docker utilities (host, images, ps, rm with guards) | Done |
| Git alias management (declarative TOML config) | Done |
| AWS commands (instances, SSH, load balancer connect) | Done |
| Platform CLIs bundle (Vercel, Supabase, Netlify, Render, Appwrite) | Done |
| Platform notes (vercel, supabase, netlify, render, appwrite) | Done |
| Platform diagnostics (`forge platform doctor/status`) | Done |
| `forge vercel` command group (doctor, env-diff, status, deploy) | Done |
| Terminal UX (TTY, dialoguer, styled rendering, clap styles) | Done |
| Skill execution design (trust, permissions, sandboxing model) | Done (design) |
| Skill execution v1 — templates + transforms (run, trust, revoke, audit) | Done |
| Skill execution v3 — script/binary execution (requires sandboxing) | Planned |
| `forge supabase` command group (doctor, migration-status, services, reset) | Done |
| `forge netlify` command group (doctor, env-diff) | Done |
| Shell profile management | Planned |
| Self-update | Planned |

## Quick start

```bash
# Build from source
cargo build --release

# Symlink for convenience
ln -sf $(pwd)/target/release/forge /usr/local/bin/forge

# Explore
forge --help
forge notes
forge system --help
forge bootstrap --list-bundles
forge bootstrap --status
```

## Installation

Requires Rust 1.84+.

```bash
git clone <repo-url> && cd my-cli
cargo build --release
```

The binary is at `target/release/forge`. No system-wide install step is required — just put it on your PATH.

## Command overview

### `forge notes [topic]`

Built-in cheatsheets for tools you use but don't memorize. 28 topics covering Kubernetes, Terraform, Helm, Docker, Git, Neovim, Claude Code, Codex, and more.

```bash
forge notes              # List all topics
forge notes list         # Same
forge notes k8s          # Kubernetes cheatsheet
forge notes claude       # Claude Code CLI reference
forge notes codex        # OpenAI Codex CLI reference
forge notes nvim         # Your Neovim config keybinds
forge notes terraform    # Terraform commands
```

### `forge system <command>`

Read-only system monitoring. No mutation, no sudo.

```bash
forge system cpu-hogs    # Top CPU consumers
forge system mem-hogs    # Top memory consumers (top)
forge system mem-hogs-ps # Top memory consumers (ps)
forge system hardware    # System hardware info
forge system ip-info     # Network interface info (en0 default)
forge system port 8080   # What's using port 8080
forge system find-pid ssh  # Find PIDs by name
forge system ps          # Your processes
forge system net-cons    # Open TCP/IP sockets
forge system top         # CPU/memory snapshot
```

### `forge file <command>`

File search and safe file mutation commands.

```bash
# Read-only
forge file find "*.toml"       # Find files by name
forge file search "TODO"       # Recursive grep (excludes .git, node_modules, target)
forge file spotlight "budget"  # macOS Spotlight search

# Mutation
forge file extract archive.tar.gz     # Extract archive (tar, zip, gz, bz2, rar, 7z)
forge file trash somefile.txt          # Move to trash (never permanently deletes)
forge file cleanup-ds                  # Find and remove .DS_Store files (with confirmation)
forge file cleanup-ds --yes            # Skip confirmation
```

**Safety:** `extract` supports 9 archive formats via explicit command dispatch (no shell interpolation). `trash` uses `trash`/`trash-put` or falls back to `~/.Trash`/`~/.local/share/Trash/files/` — it never permanently deletes. `cleanup-ds` previews all matches and requires confirmation before removing.

### `forge misc <command>`

```bash
forge misc weather              # Weather for current location
forge misc weather "New York"   # Weather for a specific location
forge misc define "ephemeral"   # Dictionary lookup
```

### `forge bootstrap`

Declarative package management via TOML bundle manifests.

```bash
# Discovery
forge bootstrap --list-bundles          # Show all bundles by tier
forge bootstrap --status                # Package inventory: installed vs expected
forge bootstrap --status --format json  # Machine-readable output
forge bootstrap --scan                  # Scan for tools beyond bundles

# Installation (brew, brew cask, npm)
forge bootstrap                         # Install default profile bundles
forge bootstrap --bundles core,go,node  # Install specific bundles
forge bootstrap --add ai-tools          # Add one bundle
forge bootstrap --all                   # Install everything
forge bootstrap --dry-run               # Preview without installing
forge bootstrap --yes                   # Skip confirmation prompt
```

**Bundle tiers:**

| Tier | Bundles | Behavior |
|------|---------|----------|
| Core | `core` | Essential shell tools. Always included. |
| Role | `go`, `node`, `python`, `python-libs`, `ruby`, `devops`, `git`, `platforms` | Opt-in per development stack. |
| Experimental | `editors`, `ios`, `elixir`, `python-ml` | Opt-in. May not apply to every machine. |
| AI Tools | `ai-tools` | Inventory and recommend. Manual entries show install instructions. |

**Supported install sources:**

| Source | Command | Notes |
|--------|---------|-------|
| Homebrew formulae | `brew install <pkg>` | Fully managed, on PATH via brew |
| Homebrew casks | `brew install --cask <pkg>` | Fully managed |
| npm globals | `npm install -g <pkg>` | Fully managed |
| Go tools | `go install <pkg>` | Installs to `$GOPATH/bin` (default `~/go/bin`). You must ensure this is on your PATH. |
| Gem packages | `gem install --user-install <pkg>` | Installs to `~/.gem/ruby/<version>/bin`. You must ensure this is on your PATH. No sudo. |
| uv tools | `uv tool install <pkg>` | Isolated venv per tool, binaries in `~/.local/bin`. You must ensure this is on your PATH. |

Other sources (pipx, pnpm, bun, composer) appear in status reports but are not yet auto-installed.

Each package is installed individually so failures are reported accurately per-package. After install, forge prints PATH hints for sources like `go` and `gem` whose binaries may not be on your default PATH.

**Manual entries** (like `claude`, `firecrawl-cli`) are never auto-installed. Bootstrap reports whether they're detected and shows install instructions.

### `forge skill`

AI skills subsystem. v1 is metadata and management only — no execution.

```bash
forge skill list                  # List discovered skills
forge skill list --tag refactoring  # Filter by tag
forge skill info <name>           # Full skill metadata
forge skill validate --all        # Validate all manifests
forge skill validate <name>       # Validate one skill
forge skill link ./my-skill       # Symlink for development
forge skill audit                 # View audit log (empty until execution is added)
```

Skills are discovered from `~/.forge/skills/` (user-global) and `.forge/skills/` (repo-scoped). Each skill has a `skill.toml` manifest defining inputs, permissions, and execution type.

### `forge shell generate-aliases`

Outputs a shell alias script to stdout. Pipe or redirect as needed.

```bash
forge shell generate-aliases              # Print to terminal
forge shell generate-aliases > aliases.sh # Save to file
```

Includes aliases for ls, kubectl, bundler, SSH, system monitoring, and macOS-specific utilities.

### `forge docker <command>`

Docker utilities with safety guards on destructive operations.

```bash
forge docker host              # Show DOCKER_HOST (or "not set")
forge docker images            # List images
forge docker ps                # List all containers
forge docker rm-images         # Remove all images (requires confirmation)
forge docker rm-images --yes   # Skip confirmation
forge docker rm-containers     # Remove all containers (requires confirmation)
forge docker rm-containers -y  # Skip confirmation
forge docker rm-by-filter "status=exited"      # Remove filtered containers
forge docker rm-by-filter "name=myapp" --yes   # Skip confirmation
```

All destructive commands (`rm-images`, `rm-containers`, `rm-by-filter`) require interactive confirmation unless `--yes`/`-y` is passed. In `--dry-run` mode, the docker commands are printed but never executed.

### `forge git <command>`

Git alias management from a declarative TOML config (`config/git-aliases.toml`).

```bash
forge git list-aliases          # Show all configured aliases
forge git setup-aliases         # Preview diff and apply to git config --global
forge --dry-run git setup-aliases  # Preview only, no changes
```

`setup-aliases` compares the config file against your current `git config --global`, shows a diff (adds, changes, unchanged), and applies only after confirmation. In `--dry-run` mode, the preview is shown but nothing is written.

### `forge aws <command>`

AWS operations using the AWS CLI as the execution backend.

```bash
forge aws instances                          # List running/stopped EC2 instances
forge aws instances --profile prod           # Use a named AWS profile
forge aws instances --query web              # Filter by Name tag substring
forge aws instances --region us-west-2       # Override region

forge aws ssh --query web-01                 # SSH to an instance matching "web-01"
forge aws ssh --query api --user ec2-user    # SSH as ec2-user
forge aws ssh --key ~/.ssh/mykey.pem         # Use a specific SSH key
forge aws ssh --profile prod --query worker  # Combine profile + filter

forge aws connect --lb-name my-alb          # List instances behind a load balancer
forge aws connect --profile prod            # Use a named profile
```

**Security notes:**
- SSH uses standard OpenSSH host key verification — no `StrictHostKeyChecking=no` override
- No shell interpolation; all AWS CLI and SSH arguments are passed as explicit arrays
- No implicit key copying or profile mutation
- If multiple instances match a query, `forge aws ssh` presents a selection prompt rather than guessing

**Differences from the old Bash version:**
- `StrictHostKeyChecking=no` removed entirely (was the default in old `mycli`)
- `--keyname` renamed to `--key` (takes a path, not just a name)
- Instance listing uses structured `--query` JMESPath + `--output text` instead of grep pipelines
- `connect` shows instances behind an ALB but does not auto-SSH; use `forge aws ssh` to connect
- All commands support `--region` override

### `forge platform <command>`

Cross-platform CLI diagnostics for deployment platforms.

```bash
forge platform status          # Show which platform CLIs are installed
forge platform doctor          # Check install, auth, and project link status
```

Checks five platforms: **Vercel**, **Supabase**, **Netlify**, **Render**, **Appwrite**.

`forge platform doctor` checks three layers for each installed CLI:
1. **Installed** — is the CLI on PATH and what version
2. **Authenticated** — are you logged in (via CLI command or token file existence)
3. **Project linked** — is the current directory linked to a project

Auth checks never read token contents — they use `whoami` commands or check for file existence only.

All platforms are also available as notes topics: `forge notes vercel`, `forge notes supabase`, etc.

**Bundle:** `forge bootstrap --add platforms` installs all five CLIs.

### `forge vercel <command>`

Vercel-specific workflows beyond what the generic platform doctor provides.

```bash
forge vercel doctor             # Deep Vercel health check (CLI, auth, project, framework, .gitignore)
forge vercel env-diff           # Compare local .env files against Vercel remote env vars
forge vercel status             # Show latest deployments
forge vercel deploy             # Deploy to preview (with confirmation)
forge vercel deploy --prod      # Deploy to production (with confirmation)
forge vercel deploy --prod -y   # Skip confirmation
```

**`forge vercel doctor`** checks:
- CLI installed and version
- Auth state (via `vercel whoami`)
- Project link (`.vercel/project.json`)
- Framework detection (Next.js, Vite, Nuxt, SvelteKit, Astro, Remix)
- `.gitignore` coverage for `.env*` and `.vercel/`

**`forge vercel env-diff`** reads local `.env.local`, `.env`, `.env.development`, `.env.development.local` and compares variable names against `vercel env ls`. Shows local-only, remote-only, and shared keys. Never displays secret values.

**`forge vercel deploy`** requires confirmation before deploying. Pre-checks that the project is linked. Supports `--dry-run`.

### `forge supabase <command>`

Supabase-specific workflows for local development and migration management.

```bash
forge supabase doctor                # Check CLI, Docker, project init/link, local stack, migrations
forge supabase migration-status      # Show local migration files and remote migration state
forge supabase services              # Show local service URLs and status
forge supabase reset                 # Reset local database (with confirmation)
forge supabase reset --yes           # Skip confirmation
```

**`forge supabase doctor`** checks:
- CLI installed and version
- Docker daemon running (required for local Supabase)
- Project initialized (`supabase/config.toml`)
- Remote project linked (`supabase/.temp/project-ref`)
- Local stack status (API, DB, Studio services)
- Local migration file count

**`forge supabase migration-status`** shows local migration files (from `supabase/migrations/`) and remote migration state via `supabase migration list`. Useful for detecting drift between local and remote schemas.

**`forge supabase services`** shows local Supabase service URLs and status via `supabase status`. Useful for quickly finding API URL, DB URL, Studio URL, etc.

**`forge supabase reset`** wraps `supabase db reset` with explicit confirmation. Destroys all local database data and recreates from migrations. Requires `--yes` to skip the prompt. Supports `--dry-run`.

### `forge netlify <command>`

Netlify-specific diagnostics and environment comparison.

```bash
forge netlify doctor            # Check CLI, auth, site link status
forge netlify env-diff          # Compare local .env files against Netlify env vars
```

**`forge netlify doctor`** checks CLI installation, auth state (via `netlify status`), and site link (via `.netlify/state.json` or `netlify status` output).

**`forge netlify env-diff`** reads local `.env` files and compares variable names against `netlify env:list`. Shows local-only, remote-only, and shared keys. Never displays secret values.

### Not yet implemented

These features are planned but not yet available:

## Global options

```
--dry-run       Preview all commands without executing anything
-v              Info-level logging
-vv             Debug-level logging
-vvv            Trace-level logging
--config <path> Override config file location
```

Dry-run is fully supported across all implemented commands, including system queries, bootstrap planning, and file operations. In dry-run mode, bootstrap status reports `[unknown]` instead of `[missing]` since package managers are not queried.

## Configuration

Three-layer TOML config: built-in defaults, `~/.forge/config.toml`, CLI flags.

Bundle manifests live in `config/bundles/*.toml`. Git aliases are declared in `config/git-aliases.toml`. The default bootstrap profile is `config/default-profile.toml`.

## Architecture

```
crates/
  forge-core/     # Library: all business logic
    commands/     # notes, system, file_ops, misc, shell_aliases, docker, git, aws, platform, vercel, supabase, netlify
    bundles/      # registry, inventory, sources, installer
    skills/       # metadata, discovery, validation, audit
    config/       # TOML schema and loading
    os/           # OsPlatform trait (macOS, Linux)
    exec/         # CommandRunner trait (real, dry-run)
  forge-cli/      # Binary: clap args, output formatting, dispatch
config/
  bundles/        # TOML bundle manifests
  default.toml    # Default config
  git-aliases.toml
shell/
  aliases.sh.tmpl # Shell alias template
```

`forge-core` contains all logic and has no direct I/O — everything goes through `CommandRunner`. `forge-cli` is a thin entry point that parses args, selects the runner (real or dry-run), and formats output.

## Development

```bash
cargo build                    # Build
cargo test                     # Run all tests (311 tests)
cargo run --bin forge -- --help  # Run locally
```

Tests cover CLI integration (help output, dry-run behavior, notes content, bootstrap planning, docker dry-run/guards, git alias listing/setup) and unit tests (install planning/execution per-package reporting, path hints, profile loading, parsers for npm, pnpm, pipx, uv, bun, composer, asdf, mise output formats, definition formatting, alias generation, git alias TOML parsing/diffing).

## Migration from mycli

If you previously used `mycli`, the key differences:

- Binary is `forge`, not `mycli`
- Commands use subcommand syntax: `forge system cpu-hogs` not `cpu_hogs`
- Shell aliases are generated, not compiled in: `forge shell generate-aliases`
- Bundle-based package management replaces imperative install scripts
- `StrictHostKeyChecking=no` removed entirely — standard SSH defaults apply
- DES3 encryption removed with no replacement (deferred until a proper design with `age` is ready)
- VPN credential injection removed (never port)
- `curl|bash` installers replaced with package manager commands

## License

MIT
