---
name: forge-environment-setup
description: Use when an AI agent needs to drive the `forge` CLI — especially to set up or provision a development environment safely (install tool bundles, configure git/shell aliases, run platform doctors). Covers forge's full command surface with a recon-first, human-gated safety model. Trigger when the user mentions forge, bootstrapping/provisioning a dev machine, installing tool bundles, or running forge commands.
---

# Using the forge CLI safely

`forge` is a developer-toolkit CLI (Rust) for bootstrapping, managing, and
extending a dev environment. This skill teaches you to use it safely and to run
a complete environment-setup workflow.

## Confirm forge is available first

```bash
forge --help
```

If `forge` is not on `PATH`, build it from source (requires Rust 1.84+):

```bash
cargo build --release
ln -sf "$(pwd)/target/release/forge" /usr/local/bin/forge
```

## Safety contract (read this before running anything)

1. **Recon before mutating.** Always run read-only commands first to understand
   current state. Present the plan to the human. Get explicit confirmation before
   any command that installs, removes, deploys, or rewrites files.
2. **Never pass `--yes` unprompted.** forge's mutating commands prompt for
   interactive confirmation. Let that prompt reach the human. Only pass `--yes`
   when the human has explicitly approved a non-interactive run.
3. **`--dry-run` is your friend.** It is a global flag and previews changes
   without executing. Use it to build the plan you show the human.
4. **Deletion rules.** `rm` on this machine is aliased to `rip`; never use
   `rm -rf`. Prefer `forge file trash` (moves to trash, never permanently
   deletes).
5. **Secrets.** Never inline plaintext credentials. Reference them as
   `op://Vault/Item/field` and resolve at runtime with `op read "<uri>"`.
6. **SSH.** Use standard OpenSSH host-key verification. Never disable it
   (no `StrictHostKeyChecking=no`).

## Environment-setup workflow

Run these phases in order. Do not skip the confirm or verify phases.

### 1. Recon (read-only)

```bash
forge bootstrap --status          # installed vs expected, drift report
forge bootstrap --scan            # tools present beyond the bundles
forge platform doctor             # platform CLI install/auth/link status (if relevant)
```

### 2. Plan (read-only)

Preview exactly what an install would change — nothing is executed:

```bash
forge bootstrap --dry-run                         # default profile
forge bootstrap --dry-run --bundles core,go,node  # specific bundles
```

### 3. Confirm

Summarize the planned changes (which bundles, which packages, which sources) and
get explicit human go-ahead before proceeding.

### 4. Apply

Run the install. Let forge's interactive confirmation fire — do not add `--yes`
unless the human approved a non-interactive run:

```bash
forge bootstrap --bundles core,go,node
```

### 5. Verify (read-only — confirms success, introduces no changes)

Confirm the setup succeeded and nothing regressed:

```bash
forge bootstrap --status          # previously-missing packages now installed; no new drift
forge platform doctor             # checks that were failing now pass
```

- Confirm newly-installed tools resolve on `PATH` (use the path hints forge
  reports, e.g. `command -v <tool>`).
- Report a clear pass/fail summary. If anything regressed or a check still
  fails, surface it explicitly — do not declare success.

Optional post-setup — gate behind confirmation before running:

```bash
forge git setup-aliases           # mutating: previews diff, applies git aliases to --global
forge shell generate-aliases > aliases.sh   # generate-aliases writes to stdout; the `> file` redirect creates the file — confirm the destination
```

## Command map

forge commands are grouped. **Read-only** commands are always safe to run for
recon. **Mutating** commands require the human gate from the safety contract.

| Group | Read-only (safe) | Mutating (gate first) |
|-------|------------------|------------------------|
| `notes` | `notes`, `notes <topic>` | — |
| `system` | all subcommands (cpu-hogs, mem-hogs, mem-hogs-ps, top, hardware, port, find-pid, ps, net-cons, ip-info, ping6) | — |
| `file` | `find`, `search`, `spotlight` | `extract`, `trash`, `cleanup-ds` |
| `bootstrap` | `--status`, `--scan`, `--list-bundles`, `--dry-run` | install (default/`--bundles`/`--add`), `--add-package` |
| `skill` | `list`, `info`, `validate`, `doctor`, `audit` | `run`, `init`, `link`, `trust`, `revoke` |
| `docker` | `host`, `images`, `ps` | `rm-images`, `rm-containers`, `rm-by-filter` |
| `git` | `list-aliases` | `setup-aliases` |
| `aws` | `instances`, `connect` | `ssh` (opens a session) |
| `platform` | `status`, `doctor` | — |
| `vercel` | `doctor`, `env-diff`, `status` | `deploy` (`--prod` deploys to production) |
| `supabase` | `doctor`, `migration-status`, `services` | `reset` (wipes local DB) |
| `netlify` | `doctor`, `env-diff` | — |
| `misc` | `weather`, `define` | — |
| `shell` | `generate-aliases` (writes to stdout) | — |

Global flags: `--dry-run` (preview, no mutations), `-v`/`-vv`/`-vvv` (verbosity),
`--config <path>` (override config file).

## Disambiguation: `forge skill` is NOT this skill

`forge skill …` is forge's **own internal subsystem** for running `skill.toml`
template/transform definitions. It is unrelated to this agent skill. Do not
confuse the two: when this document says "skill," it means these instructions;
`forge skill run <name>` runs a forge template/transform skill.
