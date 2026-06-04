# Agent guide — forge

`forge` is this repo's developer-toolkit CLI. If you are an AI agent (Gemini or
otherwise) working here, use forge to inspect and provision dev environments.

**Full guidance:** read
[`.claude/skills/forge-environment-setup/SKILL.md`](.claude/skills/forge-environment-setup/SKILL.md)
for the complete command map and the recon → plan → confirm → apply → verify
setup workflow.

## Safety contract (always)

1. **Recon before mutating.** Run read-only commands first
   (`forge bootstrap --status`, `--scan`, `forge <group> doctor`). Present the
   plan and get explicit human confirmation before any install, removal,
   deploy, or file rewrite.
2. **Never pass `--yes` unprompted.** Let forge's interactive confirmation reach
   the human. Use the global `--dry-run` flag to preview changes.
3. **Deletion:** `rm` is aliased to `rip` — never use `rm -rf`. Prefer
   `forge file trash`.
4. **Secrets:** reference as `op://Vault/Item/field` and resolve with
   `op read`. Never inline plaintext credentials.
5. **SSH:** standard OpenSSH host-key verification only; never disable it.

## Verify after setup

After applying changes, re-run read-only checks (`forge bootstrap --status`,
relevant `forge <group> doctor`) to confirm success and surface any regression
before declaring done.

## Note

`forge skill …` is forge's own template/transform subsystem — unrelated to this
agent guide.
