# forge

A developer toolkit CLI for bootstrapping, managing, and extending your dev environment. Written in Rust.

## Quick start

```bash
cargo build --release
ln -sf $(pwd)/target/release/forge /usr/local/bin/forge
forge --help
```

Requires Rust 1.84+.

## Commands

### `forge notes [topic]`

File-based cheatsheets loaded from markdown files. Add your own notes as `.md` files to `~/.forge/notes/` (user-global) or `.forge/notes/` (repo-scoped). Repo-scoped notes override user-global ones by topic name.

Each file supports optional YAML frontmatter for description and category:

```markdown
---
description: kubectl common commands
category: Infrastructure
---
  kubectl get pods -n {namespace}
  kubectl exec -it {pod} -- bash
```

```bash
forge notes              # list all topics
forge notes k8s          # show a specific topic
```

Notes directories are configurable in `config/default.toml` under `[notes]`.

### `forge system <command>`

Read-only system monitoring. No mutation, no sudo.

```bash
forge system cpu-hogs       # top CPU consumers
forge system mem-hogs       # top memory consumers
forge system hardware       # system hardware info
forge system port 8080      # what's using port 8080
forge system find-pid ssh   # find PIDs by name
forge system ps             # your processes
forge system net-cons       # open TCP/IP sockets
```

### `forge file <command>`

File search and safe file operations.

```bash
forge file find "*.toml"           # find files by name
forge file search "TODO"           # recursive grep
forge file spotlight "budget"      # macOS Spotlight search
forge file extract archive.tar.gz  # extract archive (tar, zip, gz, bz2, rar, 7z)
forge file trash somefile.txt      # move to trash (never permanently deletes)
forge file cleanup-ds              # find and remove .DS_Store files
forge file cleanup-ds --yes        # skip confirmation
```

### `forge bootstrap`

Declarative package management via TOML bundle manifests.

```bash
forge bootstrap --list-bundles          # show all bundles by tier
forge bootstrap --status                # installed vs expected
forge bootstrap --status --format json  # machine-readable output
forge bootstrap --scan                  # scan for tools beyond bundles
forge bootstrap                         # install default profile bundles
forge bootstrap --bundles core,go,node  # install specific bundles
forge bootstrap --add ai-tools          # add one bundle
forge bootstrap --dry-run               # preview without installing
```

Add packages to bundles:

```bash
forge bootstrap --add-package htop --source brew --to-bundle core
forge bootstrap --add-package tsx --source npm --to-bundle node
```

Supported install sources: Homebrew (formulae + casks), npm, Go, Gem, uv. Other sources (pipx, pnpm, bun, composer) appear in status reports but aren't auto-installed yet.

### `forge skill`

Skills subsystem — template and transform execution with trust controls.

```bash
forge skill list                  # list discovered skills
forge skill list --tag scaffold   # filter by tag
forge skill info <name>           # full skill metadata
forge skill validate --all        # validate all manifests
forge skill doctor                # check health and trust status
forge skill run <name>            # run (prompts for missing inputs)
forge skill run <name> --dry-run  # preview what would happen
forge skill trust <name>          # trust a repo-scoped skill
forge skill revoke <name>         # revoke trust
forge skill audit                 # view execution history
forge skill init <name>           # scaffold a new skill
forge skill link ./my-skill       # symlink into ~/.forge/skills/
```

Skills are discovered from `~/.forge/skills/` (user) and `.forge/skills/` (repo). See `examples/skills/` for working examples.

### `forge docker <command>`

Docker utilities with safety guards on destructive operations.

```bash
forge docker host              # show DOCKER_HOST
forge docker images            # list images
forge docker ps                # list all containers
forge docker rm-images         # remove all images (requires confirmation)
forge docker rm-containers     # remove all containers (requires confirmation)
forge docker rm-by-filter "status=exited"  # remove filtered containers
```

All destructive commands require confirmation unless `--yes` is passed.

### `forge git <command>`

Git alias management from a declarative TOML config.

```bash
forge git list-aliases          # show all configured aliases
forge git setup-aliases         # preview diff and apply to git config --global
```

### `forge aws <command>`

AWS operations using the AWS CLI.

```bash
forge aws instances                          # list EC2 instances
forge aws instances --profile prod           # use a named AWS profile
forge aws instances --query web              # filter by Name tag
forge aws ssh --query web-01                 # SSH to a matching instance
forge aws ssh --key ~/.ssh/mykey.pem         # use a specific SSH key
forge aws connect --lb-name my-alb           # list instances behind a load balancer
```

### `forge platform <command>`

Cross-platform CLI diagnostics for Vercel, Supabase, Netlify, Render, and Appwrite.

```bash
forge platform status          # which platform CLIs are installed
forge platform doctor          # check install, auth, and project link status
```

### `forge vercel <command>`

```bash
forge vercel doctor             # health check (CLI, auth, project, framework, .gitignore)
forge vercel env-diff           # compare local .env files against Vercel remote env vars
forge vercel status             # show latest deployments
forge vercel deploy             # deploy to preview
forge vercel deploy --prod      # deploy to production
```

### `forge supabase <command>`

```bash
forge supabase doctor                # check CLI, Docker, project, local stack, migrations
forge supabase migration-status      # local vs remote migration state
forge supabase services              # local service URLs and status
forge supabase reset                 # reset local database (with confirmation)
```

### `forge netlify <command>`

```bash
forge netlify doctor            # check CLI, auth, site link
forge netlify env-diff          # compare local .env files against Netlify env vars
```

### `forge misc <command>`

```bash
forge misc weather              # weather for current location
forge misc weather "New York"   # weather for a specific location
forge misc define "ephemeral"   # dictionary lookup
```

### `forge shell generate-aliases`

Outputs shell aliases to stdout. Covers ls, kubectl, bundler, SSH, system monitoring, and macOS utilities.

```bash
forge shell generate-aliases > aliases.sh
```

## Global options

```
--dry-run       Preview without executing anything
-v / -vv / -vvv Logging verbosity (info / debug / trace)
--config <path> Override config file location
```

## Configuration

Three-layer TOML config: built-in defaults, `~/.forge/config.toml`, CLI flags.

Bundle manifests live in `config/bundles/*.toml`. Git aliases in `config/git-aliases.toml`. Default bootstrap profile in `config/default-profile.toml`.

## Architecture

```
crates/
  forge-core/     # library: all business logic
  forge-cli/      # binary: clap args, dispatch, output formatting
config/           # TOML bundle manifests, git aliases, defaults
shell/            # shell alias template
```

`forge-core` has no direct I/O — everything goes through a `CommandRunner` trait (real or dry-run). `forge-cli` is a thin entry point.

## Development

```bash
cargo build
cargo test
cargo run --bin forge -- --help
```

## License

MIT

<img src="https://github-tracker-liart.vercel.app/api/pixel?source=darwinz/forge" width="1" height="1" />
