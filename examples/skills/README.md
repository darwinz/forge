# Example Skills

These are example skills demonstrating forge's skill execution model.

## Installation

Copy any skill directory to `~/.forge/skills/`:

```bash
cp -r examples/skills/scaffold-api ~/.forge/skills/
cp -r examples/skills/add-gitignore-entries ~/.forge/skills/
```

## Examples

### `scaffold-api` — Template skill

Generates a REST API route with types and tests from templates.

```bash
forge skill info scaffold-api
forge skill run scaffold-api --input name=users --input method=GET --dry-run
forge skill run scaffold-api --input name=users --input method=GET
```

**Type:** `template`
**Creates:** `src/api/{name}/route.ts`, `src/api/{name}/types.ts`, `src/api/{name}/route.test.ts`
**Inputs:** `name` (required), `method` (optional, defaults to GET in templates)

### Adding packages to bundles

For managing forge's own bundle manifests, use the built-in `--add-package` command instead of a skill:

```bash
forge bootstrap --add-package htop --source brew --to-bundle core --dry-run
forge bootstrap --add-package typescript --source npm --to-bundle node
forge bootstrap --add-package mytool --source manual --to-bundle devops \
  --instructions "Download from https://example.com" \
  --check-command "mytool --version"
```

Sources: `brew`, `cask`, `npm`, `go`, `gem`, `uv`, `manual`

### `add-gitignore-entries` — Transform skill (line_append)

Appends common ignore patterns to `.gitignore`, skipping lines that already exist.

```bash
forge skill info add-gitignore-entries
forge skill run add-gitignore-entries --dry-run
forge skill run add-gitignore-entries
```

**Type:** `transform` (line_append with `skip_duplicates = true`)
**Modifies:** `.gitignore`
**Safe:** Lines already present are not duplicated.

## Creating Your Own Skills

A skill is a directory with a `skill.toml` manifest:

```
~/.forge/skills/
  my-skill/
    skill.toml          # Manifest (required)
    templates/           # For template skills
      file.tmpl
    transform.toml       # For transform skills (optional)
```

### Template skill manifest

```toml
[skill]
name = "my-skill"
version = "1.0.0"
description = "What it does"

[skill.inputs.name]
type = "string"
required = true
description = "Input description"

[skill.permissions]
read_files = false
write_files = true

[skill.execution]
type = "template"
entrypoint = "templates/"

[[skill.execution.outputs]]
template = "file.tmpl"
destination = "output/{{ name }}.ts"
overwrite = false
```

Templates use `{{ variable }}` syntax for input substitution.

### Transform skill manifest

```toml
[skill]
name = "my-transform"
version = "1.0.0"
description = "What it does"

[skill.permissions]
read_files = true
write_files = true

[skill.execution]
type = "transform"
entrypoint = "transform.toml"

[[skill.execution.transforms]]
file = "target-file.json"
operation = "json_merge"     # or: toml_merge, line_append, line_prepend

[skill.execution.transforms.value]
key = "value"                # For json_merge / toml_merge

# Or for line operations:
# lines = ["line 1", "line 2"]
# skip_duplicates = true
```

### Supported transform operations

| Operation | Target format | Behavior |
|-----------|--------------|----------|
| `json_merge` | JSON | Deep recursive merge. Existing keys preserved, scalar conflicts: patch wins. |
| `toml_merge` | TOML | Deep recursive merge. Same semantics as json_merge. |
| `line_append` | Any text | Append lines to end of file. |
| `line_prepend` | Any text | Prepend lines to start of file. |

Line operations support `skip_duplicates = true` to avoid adding lines that already exist.

### V1 limitations

- Only `template` and `transform` execution types are supported
- `execute_commands` and `network` permissions are declared but rejected at runtime
- Script, binary, and WASM execution is deferred to a future version
