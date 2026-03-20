# Platform CLI Integration Plan

> Design document for first-class platform/devops CLI integrations in `forge`.
> Created: 2026-03-20

## Scope

This plan covers five platform CLIs and how they fit into `forge`:

| CLI | Install method | Current state in forge |
|-----|---------------|----------------------|
| **Vercel CLI** (`vercel`) | `npm i -g vercel` | Listed as npm package in `node` bundle |
| **Supabase CLI** (`supabase`) | `brew install supabase` | Listed as brew formula in `devops` bundle |
| **Render CLI** (`render`) | `brew install render` | Not present |
| **Netlify CLI** (`netlify`) | `npm i -g netlify-cli` | Listed as npm package in `node` bundle |
| **Appwrite CLI** (`appwrite`) | `npm i -g appwrite-cli` | Not present |

## Design Principles

1. **Use the official CLI as the backend** — do not reimplement platform API calls
2. **No thin wrappers** — do not rename existing CLI commands for the sake of wrapping
3. **Add wrappers only where forge adds value**: safety, multi-step workflows, cross-platform status, diagnostics
4. **Notes before wrappers** — every platform gets a cheatsheet; only some need command groups
5. **Detect before install** — inventory/status should recognize these CLIs regardless of how they were installed

## Bundle Restructuring

### New bundle: `platforms`

Move platform CLIs out of `node` and `devops` into a dedicated `platforms` bundle. These are deployment/hosting platforms, not language tools or infrastructure primitives.

```toml
# config/bundles/platforms.toml
[bundle]
name = "platforms"
description = "Deployment platforms — Vercel, Supabase, Netlify, Render, Appwrite"
tier = "role"

[brew]
formulae = ["supabase", "render"]

[npm]
packages = ["vercel", "netlify-cli", "appwrite-cli"]
```

**What changes in existing bundles:**
- Remove `vercel` and `netlify-cli` from `node.toml` npm packages
- Remove `supabase` from `devops.toml` brew formulae
- `node` bundle stays focused on language/dev tooling (typescript, eslint, prettier, nodemon)
- `devops` bundle stays focused on infrastructure (awscli, k8s, terraform, doctl, vaulted)

**Why a separate bundle:**
- Platform CLIs are optional regardless of language stack — you might use Node without Vercel
- They share a common integration pattern (login → link → deploy)
- Grouping them makes `forge bootstrap --status` reports more meaningful
- Users can `forge bootstrap --add platforms` without pulling in all of devops

### Default profile update

Add `platforms` to `config/default-profile.toml` only if the user opts in — do not add it to the default set. Platform CLIs are project-specific.

## Integration Layers Per CLI

### Layer definitions

| Layer | Description | Effort |
|-------|-------------|--------|
| **Inventory** | `forge bootstrap --status` detects if installed and reports version | Already works via bundle manifest |
| **Install** | `forge bootstrap --add platforms` installs via brew/npm | Already works for supported sources |
| **Notes** | `forge notes <platform>` shows a cheatsheet | New topic per platform |
| **Command group** | `forge <platform> <subcommand>` adds higher-level workflows | Selective — only where forge adds real value |

### Per-CLI breakdown

#### Vercel CLI

| Layer | Support | Notes |
|-------|---------|-------|
| Inventory | Install detection via npm | Already tracked when in `platforms` bundle |
| Install | `npm i -g vercel` | Supported via npm source |
| Notes | `forge notes vercel` | Deploy, env, domains, link, dev server |
| Command group | **Yes — `forge vercel`** | See below |

**Recommended `forge vercel` commands:**
- `forge vercel status` — project link state, latest deployment, env var summary (wraps `vercel inspect` + `vercel env ls`)
- `forge vercel doctor` — check login state, project link, env completeness, framework detection
- `forge vercel deploy` — guided deploy with preview/production choice, dry-run support
- `forge vercel env-diff` — compare local `.env*` against Vercel env vars (safety-critical — catches missing vars before deploy)

**Why:** Vercel has a multi-step workflow (link → env pull → dev/build/deploy) where forge can reduce friction and catch misconfiguration. The env-diff command alone justifies a command group.

**Auth/security:** Vercel CLI stores tokens in `~/.local/share/com.vercel.cli/`. `forge vercel doctor` should check login state without storing or reading tokens directly.

#### Supabase CLI

| Layer | Support | Notes |
|-------|---------|-------|
| Inventory | Install detection via brew | Already tracked |
| Install | `brew install supabase` | Supported via brew source |
| Notes | `forge notes supabase` | Init, start, db push/pull, migrations, functions |
| Command group | **Yes — `forge supabase`** | See below |

**Recommended `forge supabase` commands:**
- `forge supabase status` — local stack status (running containers, ports, migration state)
- `forge supabase doctor` — check login, project link, Docker running, local vs remote schema drift
- `forge supabase reset` — guided local reset with confirmation (wraps `supabase db reset` with guards)

**Why:** Supabase has Docker-dependent local development with migration state that is easy to get wrong. A doctor command can catch common issues (Docker not running, orphaned containers, schema drift).

**Auth/security:** Supabase CLI stores access tokens in `~/.supabase/`. Doctor should verify login state without reading token contents.

#### Netlify CLI

| Layer | Support | Notes |
|-------|---------|-------|
| Inventory | Install detection via npm | Already tracked |
| Install | `npm i -g netlify-cli` | Supported via npm source |
| Notes | `forge notes netlify` | Deploy, link, env, dev, functions, forms |
| Command group | **No — notes + bundle only** | See below |

**Why no command group:** Netlify's CLI is already well-designed for interactive use. The deploy workflow is simpler than Vercel's (single command, auto-detects framework). A `forge netlify` group would mostly be thin wrappers.

**Exception:** If we later add a cross-platform `forge deploy` command that works across Vercel/Netlify/Render, Netlify would be included as a backend.

#### Render CLI

| Layer | Support | Notes |
|-------|---------|-------|
| Inventory | Install detection via brew | Tracked when in `platforms` bundle |
| Install | `brew install render` | Supported via brew source |
| Notes | `forge notes render` | Deploy, services, env, logs, blueprints |
| Command group | **No — notes + bundle only** | See below |

**Why no command group:** Render's CLI is relatively new and focused. The service model (blueprints + dashboard) doesn't have the same multi-step workflow complexity as Vercel or Supabase.

#### Appwrite CLI

| Layer | Support | Notes |
|-------|---------|-------|
| Inventory | Install detection via npm | Tracked when in `platforms` bundle |
| Install | `npm i -g appwrite-cli` | Supported via npm source |
| Notes | `forge notes appwrite` | Init, deploy, functions, databases, auth, storage |
| Command group | **No — notes + bundle only for now** | See below |

**Why no command group now:** Appwrite's CLI has a clear init → deploy flow, but the ecosystem is less widely adopted than Vercel/Supabase. Worth revisiting if usage increases. A `forge appwrite doctor` could add value later (checking Docker, project link, function deployment state).

## Auth/Security Considerations

| Platform | Token storage | forge guidance |
|----------|--------------|----------------|
| Vercel | `~/.local/share/com.vercel.cli/auth.json` | Never read token contents; check file existence for login state |
| Supabase | `~/.supabase/access-token` | Same — existence check only |
| Netlify | `~/.netlify/config.json` | Same |
| Render | OAuth via browser | Check `render whoami` exit code |
| Appwrite | `~/.appwrite/prefs.json` | Same — existence check only |

**Rules:**
- `forge` never reads, copies, or logs authentication tokens
- Login state checks use CLI commands (`vercel whoami`, `supabase projects list`) or file existence, not file contents
- `forge` never stores platform credentials in its own config

## Implementation Order

### Immediate (can ship now)

1. **Create `platforms` bundle** — move vercel, netlify-cli, supabase out of node/devops
2. **Add Render and Appwrite** to the new bundle
3. **Update default-profile.toml** — do not add `platforms` to defaults

### Next slice (notes)

4. **`forge notes vercel`** — deploy, env, domains, link, project settings, dev
5. **`forge notes supabase`** — init, start, migrations, functions, auth, storage
6. **`forge notes netlify`** — deploy, link, env, dev, functions, forms
7. **`forge notes render`** — deploy, services, blueprints, env, logs
8. **`forge notes appwrite`** — init, deploy, functions, databases, auth

### Future slices (command groups)

9. **`forge vercel` command group** — status, doctor, deploy, env-diff
10. **`forge supabase` command group** — status, doctor, reset

## Summary Recommendations

### 1. Add to bundles immediately
All five: **Vercel, Supabase, Netlify, Render, Appwrite** — in a new `platforms` bundle.

### 2. Add notes topics next
All five should get `forge notes` entries. Priority order:
1. Vercel (most widely used in this stack)
2. Supabase (complex local dev workflow)
3. Netlify (common alternative)
4. Render (growing adoption)
5. Appwrite (niche but self-hostable)

### 3. Dedicated `forge` command groups
- **Yes:** Vercel, Supabase — both have multi-step workflows where forge adds real value (env-diff, doctor, guided deploy, local stack diagnostics)
- **No (for now):** Netlify, Render, Appwrite — their CLIs are sufficient; revisit if a cross-platform `forge deploy` emerges
