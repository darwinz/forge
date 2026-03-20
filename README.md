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
| Notes/cheatsheets (28 topics) | Done |
| System monitoring (read-only) | Done |
| File search (`find`, `search`, `spotlight`) | Done |
| Misc (`weather`, `define`) | Done |
| Shell alias generation | Done |
| Bundle manifests and registry | Done |
| Package inventory/status/drift reporting | Done |
| Environment scan (pipx, uv, pnpm, bun, asdf, mise, bin dirs) | Done |
| Bootstrap install (brew, brew cask, npm, go, gem) | Done |
| Skills subsystem (metadata, discovery, validation) | Done (no execution) |
| `--dry-run` across all commands | Done |
| Docker utilities (host, images, ps, rm with guards) | Done |
| Git alias management (declarative TOML config) | Done |
| AWS commands | Planned |
| Shell profile management | Planned |
| Self-update | Planned |
| Skill execution / sandbox | Planned |

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

File search commands (read-only subset implemented).

```bash
forge file find "*.toml"       # Find files by name
forge file search "TODO"       # Recursive grep (excludes .git, node_modules, target)
forge file spotlight "budget"  # macOS Spotlight search
```

`extract`, `trash`, `cleanup-ds`, `mkdir-cd` are defined but not yet implemented (require mutation).

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
| Role | `go`, `node`, `python`, `ruby`, `devops`, `git` | Opt-in per development stack. |
| Experimental | `editors`, `ios`, `elixir` | Opt-in. May not apply to every machine. |
| AI Tools | `ai-tools` | Inventory and recommend. Manual entries show install instructions. |

**Supported install sources:**

| Source | Command | Notes |
|--------|---------|-------|
| Homebrew formulae | `brew install <pkg>` | Fully managed, on PATH via brew |
| Homebrew casks | `brew install --cask <pkg>` | Fully managed |
| npm globals | `npm install -g <pkg>` | Fully managed |
| Go tools | `go install <pkg>` | Installs to `$GOPATH/bin` (default `~/go/bin`). You must ensure this is on your PATH. |
| Gem packages | `gem install --user-install <pkg>` | Installs to `~/.gem/ruby/<version>/bin`. You must ensure this is on your PATH. No sudo. |

Other sources (pipx, uv, pnpm, bun, composer) appear in status reports but are not yet auto-installed.

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

### Not yet implemented

These command groups are defined in the CLI but return placeholder messages:

- `forge aws` — AWS operations (Phase 5)

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
    commands/     # notes, system, file_ops, misc, shell_aliases, docker, git
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
cargo test                     # Run all tests (119 tests)
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
