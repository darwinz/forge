# forge agent skill

forge ships an **AI agent skill** that teaches coding agents — Claude, Codex,
and Gemini — to drive the `forge` CLI to set up and manage a development
environment *safely*. It is recon-first and human-gated: the agent inspects the
current state, shows you a plan, and waits for your go-ahead before anything is
installed, removed, deployed, or rewritten.

This is **not** the same as `forge skill` (forge's own internal
template/transform subsystem). This page is about the AI-agent-facing
instruction skill that lives under `.claude/skills/`.

## What it does for you

When you ask an agent to "set up this dev environment," "bootstrap my machine,"
or run forge commands, the skill gives the agent a consistent, safe playbook
instead of letting it improvise:

- A **command map** of forge's full surface, with every command tagged
  read-only (safe to run for recon) or mutating (requires your confirmation).
- A **five-phase setup workflow**: recon → plan → confirm → apply → verify.
- A **safety contract** that encodes guardrails (no unprompted `--yes`,
  `--dry-run` previews, trash instead of delete, 1Password references for
  secrets, standard SSH host-key verification).

The result: the agent does the legwork (inspecting drift, previewing installs,
re-checking after) while you keep the final say on anything that changes your
machine.

## Supported tools

| Tool | How it loads the skill |
|------|------------------------|
| **Claude** (Claude Code) | Auto-discovers `.claude/skills/forge-environment-setup/SKILL.md`. No setup needed. |
| **Codex** | Reads `AGENTS.md` at the repo root, which carries the safety contract inline and points to the canonical skill. |
| **Gemini** (Gemini CLI) | Reads `GEMINI.md` at the repo root, same pattern as `AGENTS.md`. |

All three drive `forge` through their shell/Bash tool — there is no per-tool
command translation. `SKILL.md` is the single source of truth; the two pointer
files exist only so Codex and Gemini have a native filename to auto-load.

## How it triggers

The skill's `description` frontmatter tells the agent when to reach for it.
You'll generally see it engage when you mention forge, provisioning or
bootstrapping a dev machine, installing tool bundles, or running forge commands.
With Claude Code you can also invoke it explicitly by name
(`forge-environment-setup`).

## The safety model

### Setup workflow

1. **Recon** (read-only): `forge bootstrap --status`, `--scan`, relevant
   `forge <group> doctor` — understand what's installed and what's drifted.
2. **Plan** (read-only): `forge bootstrap --dry-run [--bundles …]` — preview the
   exact changes an install would make. Nothing executes.
3. **Confirm**: the agent summarizes the planned changes and waits for your
   explicit go-ahead.
4. **Apply**: the install runs with forge's interactive confirmation intact (no
   unprompted `--yes`).
5. **Verify** (read-only): re-run `--status` and the doctors to confirm the
   previously-missing tools are installed, nothing regressed, and new tools
   resolve on `PATH`. The agent reports a clear pass/fail rather than assuming
   success.

### Safety contract

The skill instructs agents to:

- Run **read-only recon first** and get explicit confirmation before any mutation.
- **Never pass `--yes` unprompted** — let forge's interactive prompt reach you.
- Use the global **`--dry-run`** flag to build the plan they show you.
- **Never delete** with `rm -rf`; prefer `forge file trash`.
- Reference **secrets** as `op://Vault/Item/field` (resolved via `op read`),
  never inline plaintext.
- Use **standard OpenSSH host-key verification** (never `StrictHostKeyChecking=no`).

The full command map and the complete contract live in
[`.claude/skills/forge-environment-setup/SKILL.md`](../.claude/skills/forge-environment-setup/SKILL.md).

## What bootstrap installs

`forge bootstrap` is declarative — every package comes from a TOML **bundle
manifest** in [`config/bundles/`](../config/bundles/), grouped by install source
(Homebrew formulae/casks, npm, Go, Gem, uv). The manifests are the source of
truth; the lists below are a snapshot to give you a sense of scope.

### Default profile

With no arguments, `forge bootstrap` installs the bundles listed in
[`config/default-profile.toml`](../config/default-profile.toml):
**core, git, node, python, editors**. Override per machine with, e.g.,
`forge bootstrap --bundles core,go,node`.

Those defaults pull in, for example:

- **core** (required) — modern shell tooling: `bat`, `fd`, `fzf`, `ripgrep`,
  `jq`, `lsd`, `zoxide`, `tmux`, `dust`, `just`, `tldr`, `oh-my-posh`, … plus the
  Hack Nerd Font.
- **git** — `git`, `git-flow`, `gh`, `hub`, `forgit`, and the configured git aliases.
- **node** — `node`, `yarn`, plus global npm CLIs `typescript`, `eslint`,
  `prettier`, `nodemon`.
- **python** — `pyenv`, `pyenv-virtualenv`, `uv`, plus `ipython` and `pytest`
  installed as isolated `uv` tools.
- **editors** — `neovim`, `ctags`, and the VS Code and Cursor casks.

### All bundles

| Bundle | What it sets up | Tier |
|--------|-----------------|------|
| `core` | Shell tools, search, system utilities (required) | core |
| `git` | Git — aliases, flow, interactive tools | role |
| `node` | Node.js — essential global CLIs | role |
| `python` | Python toolchain — pyenv, uv, virtualenv | role |
| `editors` | Code editors and terminal tools | experimental |
| `go` | Go — goenv, language server, linters, debugger, REPL | role |
| `ruby` | Ruby — rbenv, ruby-build | role |
| `elixir` | Elixir/Erlang | experimental |
| `ios` | iOS — CocoaPods | experimental |
| `devops` | Kubernetes, Terraform, Docker, AWS, cloud CLIs | role |
| `platforms` | Vercel, Supabase, Netlify, Render, Appwrite | role |
| `ai-tools` | AI development CLIs | ai-tools |
| `python-libs` | Python libraries for your project virtualenv | role |
| `python-ml` | Python ML libraries (tensorflow, pytorch, …) | experimental |

See the individual files in [`config/bundles/`](../config/bundles/) for the exact
package list of each bundle.

### See it before you install

The skill's recon phase surfaces all of this without changing anything:

```bash
forge bootstrap --list-bundles   # every bundle, grouped by tier
forge bootstrap --status         # what's installed vs expected for your profile
forge bootstrap --dry-run        # exactly what an install would add
```

## For contributors

### Structure

```
.claude/skills/forge-environment-setup/SKILL.md   # canonical — Claude auto-discovers
AGENTS.md                                          # Codex entry point (root)
GEMINI.md                                          # Gemini entry point (root)
```

`SKILL.md` carries YAML frontmatter (`name`, `description`) plus the full body:
availability check, safety contract, setup workflow, command map, and a
disambiguation note about `forge skill`. `AGENTS.md` and `GEMINI.md` are thin —
they **intentionally duplicate** the short, high-value safety contract inline
(because Codex/Gemini read those files directly) and point to `SKILL.md` for the
command map and workflow detail. This avoids three-way duplication of the bulk
while giving each tool a native entry point.

### Updating the skill

1. Edit `SKILL.md` as the source of truth.
2. If you change the **safety contract**, mirror the change in the inline
   contracts in `AGENTS.md` and `GEMINI.md` so all three stay consistent.
3. Keep the frontmatter valid — `name` must remain `forge-environment-setup`
   and `description` must be non-empty.

### The accuracy contract

Every forge command and flag named in `SKILL.md` **must exist** in the CLI
definition at `crates/forge-cli/src/args.rs` (the clap source of truth). When you
add, rename, or remove a forge command, update `SKILL.md`'s command map to match,
including the read-only vs mutating classification. clap maps CamelCase enum
variants to kebab-case subcommands (e.g. `CleanupDs` → `cleanup-ds`,
`MigrationStatus` → `migration-status`). When in doubt, reconcile the command map
against `args.rs` variant by variant before committing.
