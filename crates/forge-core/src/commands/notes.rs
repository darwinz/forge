use crate::error::{ForgeError, ForgeResult};

/// Available notes topics
pub static TOPICS: &[(&str, &str)] = &[
    ("screen", "screen session management"),
    ("curl-form", "remote form post with file upload"),
    ("forgit", "interactive git tool (forgit) keybinds"),
    ("k8s", "kubectl common commands"),
    ("helm", "helm chart management"),
    ("terraform", "terraform CLI commands"),
    ("gcloud", "gcloud CLI commands"),
    ("packer", "packer build commands"),
    ("blackbox", "GPG file encryption/decryption with blackbox"),
    ("jo", "JSON CLI tool (jo) usage"),
    ("termtosvg", "terminal recording to SVG"),
    ("httpie", "HTTPie CLI examples"),
    ("lsd", "ls replacement (lsd)"),
    ("rip", "rm replacement (rip)"),
    ("zoxide", "directory jumping (z/zoxide)"),
    ("dust", "disk usage analyzer (dust)"),
    ("fd", "find replacement (fd)"),
    ("sd", "sed replacement (sd)"),
    ("procs", "ps replacement (procs)"),
    ("bottom", "system monitor (bottom/btm)"),
    ("topgrade", "system upgrade tool (topgrade)"),
    ("broot", "tree alternative (broot)"),
    ("tokei", "code statistics (tokei)"),
    ("eva", "CLI calculator REPL (eva)"),
    ("op", "1Password CLI (op)"),
    ("claude", "Claude Code CLI — AI coding agent"),
    ("codex", "OpenAI Codex CLI — AI coding agent"),
    ("nvim", "Neovim config — plugins, keybinds, Go dev, LSP"),
    ("vercel", "Vercel CLI — deploy, env, domains, dev server"),
    (
        "supabase",
        "Supabase CLI — local dev, migrations, functions",
    ),
    ("netlify", "Netlify CLI — deploy, dev, env, functions"),
    ("render", "Render CLI — services, deploys, env, logs"),
    (
        "appwrite",
        "Appwrite CLI — projects, databases, functions, auth",
    ),
];

/// List all available notes topics
pub fn list_topics() -> Vec<(&'static str, &'static str)> {
    TOPICS.to_vec()
}

/// Get notes content for a topic
pub fn get_notes(topic: &str) -> ForgeResult<&'static str> {
    match topic {
        "screen" => Ok(NOTES_SCREEN),
        "curl-form" => Ok(NOTES_CURL_FORM),
        "forgit" => Ok(NOTES_FORGIT),
        "k8s" => Ok(NOTES_K8S),
        "helm" => Ok(NOTES_HELM),
        "terraform" => Ok(NOTES_TERRAFORM),
        "gcloud" => Ok(NOTES_GCLOUD),
        "packer" => Ok(NOTES_PACKER),
        "blackbox" => Ok(NOTES_BLACKBOX),
        "jo" => Ok(NOTES_JO),
        "termtosvg" => Ok(NOTES_TERMTOSVG),
        "httpie" => Ok(NOTES_HTTPIE),
        "lsd" => Ok(NOTES_LSD),
        "rip" => Ok(NOTES_RIP),
        "zoxide" => Ok(NOTES_ZOXIDE),
        "dust" => Ok(NOTES_DUST),
        "fd" => Ok(NOTES_FD),
        "sd" => Ok(NOTES_SD),
        "procs" => Ok(NOTES_PROCS),
        "bottom" => Ok(NOTES_BOTTOM),
        "topgrade" => Ok(NOTES_TOPGRADE),
        "broot" => Ok(NOTES_BROOT),
        "tokei" => Ok(NOTES_TOKEI),
        "eva" => Ok(NOTES_EVA),
        "op" => Ok(NOTES_OP),
        "claude" => Ok(NOTES_CLAUDE),
        "codex" => Ok(NOTES_CODEX),
        "nvim" => Ok(NOTES_NVIM),
        "vercel" => Ok(NOTES_VERCEL),
        "supabase" => Ok(NOTES_SUPABASE),
        "netlify" => Ok(NOTES_NETLIFY),
        "render" => Ok(NOTES_RENDER),
        "appwrite" => Ok(NOTES_APPWRITE),
        _ => Err(ForgeError::NotesTopicNotFound(topic.to_string())),
    }
}

static NOTES_SCREEN: &str = "\
Create a new screen session:
  screen -S some_name

Detach from screen:
  Ctrl + A and then d

View a shared screen:
  screen -x some_name

Go into previously attached screen:
  screen -r

Other commands:
  screen -ls";

static NOTES_CURL_FORM: &str = "\
Post form, including file, remotely with the following command:
  curl -v -F \"FileInputName=@/path/to/file.gif;OtherInputName=Value;OtherRequestK=Val;\" http://formurl.com";

static NOTES_FORGIT: &str = "\
Commands:
  gd   - Interactive git diff viewer
  grh  - Interactive git reset HEAD <file> selector
  gcf  - Interactive git checkout <file> selector
  gcb  - Interactive git checkout <branch> selector
  gco  - Interactive git checkout <commit> selector
  gss  - Interactive git stash viewer
  gclean - Interactive git clean selector
  gcp  - Interactive git cherry-pick selector
  grb  - Interactive git rebase -i selector
  gfu  - Interactive git commit --fixup && rebase -i --autosquash selector

Keybinds:
  Enter       - Confirm
  Tab         - Toggle mark and move up
  Shift-Tab   - Toggle mark and move down
  ?           - Toggle preview window
  Alt-W       - Toggle preview wrap
  Ctrl-S      - Toggle sort
  Ctrl-R      - Toggle selection
  Ctrl-Y      - Copy commit hash
  Ctrl-K / P  - Selection move up
  Ctrl-J / N  - Selection move down
  Alt-K / P   - Preview move up
  Alt-J / N   - Preview move down

Aliases:
  forgit_log=glo
  forgit_diff=gd
  forgit_add=ga
  forgit_reset_head=grh
  forgit_ignore=gi
  forgit_checkout_file=gcf
  forgit_checkout_branch=gcb
  forgit_checkout_commit=gco
  forgit_clean=gclean
  forgit_stash_show=gss
  forgit_cherry_pick=gcp
  forgit_rebase=grb
  forgit_fixup=gfu";

static NOTES_K8S: &str = "\
  kubectl config get-contexts
  kubectl config set-context {context}
  kubectl -n {namespace} get pods
  kubectl -n {namespace} exec -it {pod} -- bash
  kubectl -n {namespace} get deployments
  kubectl -n {namespace} get configmaps
  kubectl -n {namespace} delete pod {pod}
  kubectl -n {namespace} port-forward {pod} {ip}:{ip}
  kubectl -n {namespace} describe pod {pod}
  kubectl -n {namespace} scale --replicas={number} deployment {deployment}
  kubectl -n {namespace} patch pvc {pvc-name} -p '{\"metadata\":{\"finalizers\": []}}' --type=merge
  kubectl cluster-info
  kubectl get secret {secret-name} -n {namespace} -o yaml
  kubectl get secret {secret-name} -n {namespace} -o yaml | grep {some-search-value}
  kubectl cp -c {container} {namespace}/{pod}:{full-remote-file-path} {local-file-path}
  kubectl cp -c {container} {local-file-path} {namespace}/{pod}:{full-remote-file-path}";

static NOTES_HELM: &str = "\
  helm search hub prometheus-community --max-col-width 100
  helm repo add prometheus-community https://prometheus-community.github.io/helm-charts
  helm repo add prometheus https://prometheus-community.github.io/helm-charts
  helm show values prometheus-community/prometheus
  helm list -n {k8s-namespace}
  helm delete prometheus -n {k8s-namespace}";

static NOTES_TERRAFORM: &str = "\
  terraform login
  terraform init
  terraform plan
  terraform plan -out out/file/path
  terraform apply -target module.path
  terraform apply \"plan/file/path\"
  terraform import module.path [...arguments]
  terraform state list
  terraform state show
  terraform state show module.path
  terraform state mv module.path new.module.path
  terraform state rm module.path
  terraform state pull
  terraform state push
  terraform validate
  terraform fmt
  terraform taint module.path

  # dangerous/caution
  terraform destroy -target module.path
  terraform force-unlock {lock-id}";

static NOTES_GCLOUD: &str = "\
  gcloud init
  gcloud init --console-only
  gcloud auth login
  gcloud config list
  gcloud container clusters get-credentials {gcloud-project-id} --zone us-central1-a
  gcloud components install docker-credential-gcr
  gcloud auth configure-docker
  gcloud container images list
  gcloud container images list --repository={hostname}/{gcr-project-id}
  gcloud container images list-tags
  gcloud container images list-tags gcr.io/true-bit-256021/{gcr-project-id}
  gcloud components install kubectl
  gcloud compute addresses list";

static NOTES_PACKER: &str = "\
  packer init {packer-hcl-file}
  packer build -var-file=./{packer-var-file} [-debug] [-on-error=ask] {packer-hcl-file}";

static NOTES_BLACKBOX: &str = "\
  blackbox_initialize
  gpg --list-keys
  blackbox_addadmin {admin-gpg-key}
  blackbox_register_new_file {filename}
  blackbox_edit_start {gpg-encrypted-file-path}
  blackbox_edit_end {edited-file}
  blackbox_diff
  blackbox_decrypt_all_files
  blackbox_shred_all_files
  blackbox_postdeploy";

static NOTES_JO: &str = "\
https://github.com/jpmens/jo/blob/master/jo.md

  jo name=Brandon
  {\"name\": \"Brandon\"}

  jo tst=1457081292 lat=12.3456 cc=FR name=\"JP Mens\" nada= coffee@T
  {\"tst\":1457081292,\"lat\":12.3456,\"cc\":\"FR\",\"name\":\"JP Mens\",\"nada\":null,\"coffee\":true}

  jo -p -a *
  [\"Makefile\", \"README.md\", ...]

  jo -p name=JP object=$(jo fruit=Orange hungry@0 point=$(jo x=10 y=20 list=$(jo -a 1 2 3 4 5))) number=17) sunday@0
  {
    \"name\": \"JP\",
    \"object\": {
      \"fruit\": \"Orange\",
      \"hungry\": false,
      \"point\": { \"x\": 10, \"y\": 20, \"list\": [1, 2, 3, 4, 5] },
      \"number\": 17
    },
    \"sunday\": false
  }";

static NOTES_TERMTOSVG: &str = "\
https://github.com/nbedos/termtosvg

  $ termtosvg
  Recording started, enter \"exit\" command or Control-D to end

  $ exit
  Recording ended, file is /tmp/termtosvg_exp5nsr4.svg";

static NOTES_HTTPIE: &str = "\
https://httpie.io/docs/cli/examples

  http PUT pie.dev/put X-API-Token:123 name=John
  http -f POST pie.dev/post hello=World
  http -v pie.dev/get

  # Use https
  https example.org

  # Build and print a request without sending it
  http --offline pie.dev/post hello=offline

  # With authentication
  http -a USERNAME POST https://api.github.com/repos/httpie/httpie/issues/83/comments body='HTTPie is awesome!'

  # Upload a file
  http pie.dev/post < files/data.json

  # Download a file
  http pie.dev/image/png > image.png

  # Using named session to persist between requests
  http --session=logged-in -a username:password pie.dev/get API-Key:123
  http --session=logged-in pie.dev/headers

  # Use query string params
  http https://api.github.com/search/repositories q==httpie per_page==1";

static NOTES_LSD: &str = "\
https://github.com/Peltoche/lsd

  ls replacement with colors, icons, and tree view";

static NOTES_RIP: &str = "\
https://github.com/nivekuil/rip

  rm replacement — moves files to graveyard instead of deleting";

static NOTES_ZOXIDE: &str = "\
https://github.com/ajeetdsouza/zoxide

Zoxide (z) remembers each time you cd into a directory and ranks them for next time.

Use <space>+<tab> to pull up the prompt.";

static NOTES_DUST: &str = "\
https://github.com/bootandy/dust

  dust                      - Show disk usage for current directory
  dust <dir>                - Show disk usage for a specific directory
  dust <dir> <another_dir>  - Compare multiple directories
  dust -p                   - Show full paths
  dust -s                   - Show apparent size (not disk usage)
  dust -n 30                - Show 30 directories (default: terminal height)
  dust -d 3                 - Show 3 levels of subdirectories
  dust -D                   - Show only directories
  dust -r                   - Reverse order of output
  dust -H                   - Print sizes in powers of 1000 (SI)
  dust -X ignore            - Ignore files/dirs named 'ignore'
  dust -x                   - Only show same filesystem
  dust -b                   - No percentages or ASCII bars
  dust -i                   - No hidden files
  dust -c                   - No colors (monochrome)
  dust -f                   - Count files instead of diskspace
  dust -t                   - Group by filetype
  dust -z 10M               - Only include files larger than 10M
  dust -e regex             - Only include files matching regex
  dust -v regex             - Exclude files matching regex";

static NOTES_FD: &str = "\
https://github.com/sharkdp/fd

Find replacement:
  fd sometext               - Search for files matching 'sometext'
  fd sometext --type f      - Search only files
  fd sometext --type d      - Search only directories
  fd '^x.*rc$'              - Regex search
  fd passwd /etc            - Search in specific root directory";

static NOTES_SD: &str = "\
https://github.com/chmln/sd

Sed replacement:
  sd text replacement dump.json
  sd '\"' \"'\" *.json >/dev/null
  echo 'lots((([]))) of special chars' | sd -s '((([])))' ''
  echo 'lorem ipsum 23   ' | sd '\\s+$' ''";

static NOTES_PROCS: &str = "\
https://github.com/dalance/procs

ps replacement:
  procs                     - List all processes
  procs zsh                 - Search by name
  procs --or 6000 60000     - Search by multiple PIDs
  procs --and docker nerd   - Match all keywords
  procs --tree              - Tree view
  procs --sortd cpu         - Sort descending by CPU
  procs --sorta cputime     - Sort ascending by CPU time";

static NOTES_BOTTOM: &str = "\
https://github.com/ClementTsang/bottom

Complementary to top:
  btm                       - Launch bottom
  btm --help                - Show help";

static NOTES_TOPGRADE: &str = "\
https://github.com/topgrade-rs/topgrade

Keeping your system up to date:
  topgrade                  - Run all upgrade steps";

static NOTES_BROOT: &str = "\
https://github.com/Canop/broot

Tree alternative with interactivity and file previewing.";

static NOTES_TOKEI: &str = "\
https://github.com/XAMPPRocky/tokei

Displays statistics about your code:
  tokei ./foo               - Stats for one directory
  tokei ./foo ./bar ./baz   - Stats for multiple directories
  tokei ./foo --exclude *.rs - Exclude by pattern";

static NOTES_EVA: &str = "\
CLI REPL calculator:
  eva                       - Launch calculator

  2 ^ 5 ^ 2                - Exponentiation
  33,554,432.0

  log(sqrt(100))            - Functions
  sin(45 + 5 * 9)
  1.0";

static NOTES_OP: &str = "\
https://github.com/1Password/shell-plugins

1Password CLI:
  op item list | grep <item name>
  op get item <item name>
  op get item <item name> --fields <field name>
  op document list | grep <item name>
  op get document <document name>";

static NOTES_CLAUDE: &str = "\
https://docs.anthropic.com/en/docs/claude-code

Claude Code — AI coding agent that runs in your terminal.

Interactive session:
  claude                            Start interactive session in current directory
  claude \"explain this codebase\"    Start with an initial prompt
  claude -c                         Continue most recent conversation
  claude -r                         Resume a previous session (interactive picker)
  claude -r {session-id}            Resume a specific session

Non-interactive (piped/scripted):
  claude -p \"what does main.rs do\"  Print response and exit
  cat file.ts | claude -p \"review this\"
  claude -p \"find all TODOs\" --output-format json

Model selection:
  claude --model sonnet             Use Sonnet (faster, cheaper)
  claude --model opus               Use Opus (most capable)
  claude --model haiku              Use Haiku (fastest)

Worktrees (isolated changes):
  claude -w                         Auto-create a git worktree for this session
  claude -w my-feature              Named worktree
  claude -w --tmux                  Worktree in a tmux/iTerm pane

Permission modes:
  claude --permission-mode plan     Read-only, no edits
  claude --permission-mode default  Ask before edits (default)
  claude --permission-mode auto     Allow pre-approved tools without asking

Tool control:
  claude --allowedTools \"Bash(git:*) Read\"  Restrict to specific tools
  claude --disallowedTools \"Write\"          Block specific tools

Multi-directory:
  claude --add-dir ../other-repo    Grant access to additional directories

MCP servers:
  claude mcp add {name} -- {command}     Add an MCP server
  claude mcp list                        List configured servers
  claude mcp remove {name}               Remove a server

Useful slash commands (inside interactive session):
  /help                             Show available commands
  /clear                            Clear conversation history
  /compact                          Summarize and compact context
  /cost                             Show token/cost usage
  /review                           Review a PR or diff
  /commit                           Commit staged changes

Workflows:
  # Review current changes
  claude -p \"review the diff\" < <(git diff)

  # Ask about code without modifying
  claude --permission-mode plan

  # Run with budget cap
  claude -p \"refactor auth module\" --max-budget-usd 5.00

  # Parallel tasks in worktrees
  claude -w feat-a \"implement feature A\" &
  claude -w feat-b \"implement feature B\" &";

static NOTES_CODEX: &str = "\
https://github.com/openai/codex

OpenAI Codex CLI — AI coding agent with sandboxed execution.

Interactive session:
  codex                             Start interactive session
  codex \"add input validation\"      Start with a prompt

Non-interactive:
  codex exec \"fix the failing test\"           Run and exit
  codex exec \"refactor auth\" -o result.md     Save last message to file
  echo \"add tests\" | codex exec -             Read prompt from stdin

Code review:
  codex review                      Review current repo changes

Resume/fork sessions:
  codex resume                      Pick a previous session to continue
  codex resume --last               Continue the most recent session
  codex fork                        Fork a previous session (new branch of conversation)
  codex fork --last                 Fork the most recent session

Model selection:
  codex -m o3                       Use o3
  codex -m o4-mini                  Use o4-mini (default)
  codex --oss                       Use local model (LM Studio / Ollama)
  codex --oss --local-provider ollama

Sandbox modes:
  codex -s read-only                Read filesystem only
  codex -s workspace-write          Write within workspace (default with --full-auto)
  codex -s danger-full-access       Unrestricted (use with caution)

Approval policies:
  codex -a untrusted                Ask before non-trivial commands
  codex -a on-request               Model decides when to ask
  codex -a never                    Never ask (for non-interactive/CI)
  codex --full-auto                 Shortcut: -a on-request + -s workspace-write

Working directory:
  codex -C /path/to/project         Run in a different directory
  codex --add-dir ../shared-lib     Grant write access to additional dirs

Configuration:
  codex -c model=\"o3\"               Override config value
  codex -p my-profile               Use a named profile from ~/.codex/config.toml

Web search:
  codex --search                    Enable live web search tool

MCP servers:
  codex mcp add {name} -- {command}     Add an MCP server
  codex mcp list                        List configured servers
  codex mcp remove {name}               Remove a server

Apply changes from a previous run:
  codex apply                       Apply last diff as git apply
  codex apply --last                Apply from most recent session

Workflows:
  # Quick autonomous task
  codex --full-auto \"add error handling to all API routes\"

  # Safe read-only investigation
  codex -s read-only \"explain the auth flow in this codebase\"

  # CI/scripted usage
  codex exec -a never \"generate changelog from recent commits\" -o CHANGELOG.md

  # Review before merging
  codex review";

static NOTES_NVIM: &str = "\
Leader: ,

Navigation:
  ,n              Toggle nvim-tree file explorer
  ,g              Telescope find files (dropdown)
  s / S           Flash jump / treesitter jump
  :Telescope live_grep        Search file contents
  :Telescope buffers          List open buffers
  :Telescope git_status       Git changed files
  :Telescope diagnostics      LSP diagnostics
  :BufferLinePick             Pick buffer by letter
  :BufferLineCycleNext/Prev   Next/prev buffer tab

nvim-tree (inside tree):
  a  create    d  delete    r  rename    x  cut    p  paste    R  refresh

Editing:
  gcc / gbc     Toggle line / block comment
  gc / gb       Comment with motion or visual selection
  gcO / gco     Comment line above / below
  <C-s>         Expand snippet or jump to next placeholder (insert mode)
  jk / ii       Escape to normal mode

Git:
  ,gs           Fugitive git status
  ]c / [c       Next / prev git hunk (gitsigns)
  ]n / [n       Next / prev conflict marker
  :Gitsigns stage_hunk / reset_hunk / preview_hunk / blame_line
  :Git blame    Inline blame
  :Gdiffsplit   Side-by-side diff
  :GBrowse      Open on GitHub (vim-rhubarb)

Go (go.nvim):
  ,t            Test function under cursor
  ,tp           Test current package
  ,tf           Test current file
  ,ta           Test all
  ,ts           Select test function
  ,tc           Close test pane
  ]m / [m       Next / prev method
  :GoFmt        Format          :GoImports     Organize imports
  :GoLint       Lint            :GoFillStruct  Fill struct fields
  :GoAddTag     Add struct tags :GoRmTag       Remove tags
  :GoIfErr      Insert if err   :GoImpl        Generate interface impl
  :GoCoverage   Test coverage   :GoDoc         Browse docs
  :GoRename     Rename symbol   :GoDebug       Start DAP debugger

LSP:
  gd            Go to definition
  gD            Go to declaration
  gr            References
  gi            Go to implementation
  K             Hover documentation
  ,ra           Rename
  ,ca           Code action
  [d / ]d       Prev / next diagnostic
  ,f            Floating diagnostic

AI:
  Copilot: suggestions via nvim-cmp (ghost text off)
    :Copilot status / enable / disable
  Avante: AI chat (Claude sonnet)
    :AvanteAsk / :AvanteChat / :AvanteToggle
  cc            Open CodeCompanion chat

UI:
  ,m            Dismiss all notifications (noice.nvim)
  :Noice        Message history
  :Noice last / errors / all

Formatting:
  Lua: stylua      Go: goimports on save (gopls, gofumpt enabled)
  Format-on-save off globally. Manual: :lua require(\"conform\").format()

Plugin management (lazy.nvim):
  :Lazy              Dashboard
  :Lazy sync         Install + update + clean
  :Lazy update       Update only
  :Lazy profile      Startup profiling
  :TSUpdate          Update treesitter parsers
  :checkhealth       Diagnose issues

Plugins: nvim-tree, telescope, flash, bufferline, Comment, neosnippet,
  conform, gitsigns, fugitive, go.nvim, copilot, avante, noice, notify,
  markview, wakatime, navigator, lazy.nvim, treesitter";

static NOTES_VERCEL: &str = "\
https://vercel.com/docs/cli

Setup:
  vercel login                        Authenticate
  vercel link                         Link current directory to a Vercel project
  vercel env pull .env.local          Pull environment variables to local file
  vercel whoami                       Show current user

Development:
  vercel dev                          Start local dev server (mirrors Vercel environment)
  vercel build                        Run production build locally

Deploy:
  vercel                              Deploy to preview
  vercel --prod                       Deploy to production
  vercel deploy --prebuilt            Deploy pre-built output (fast, for CI)
  vercel promote <url>                Promote a preview deployment to production
  vercel rollback                     Roll back to previous production deployment

Environment variables:
  vercel env ls                       List all env vars
  vercel env add <name>               Add env var (interactive)
  vercel env rm <name>                Remove env var
  vercel env pull                     Sync remote env vars to .env.local

Domains:
  vercel domains ls                   List domains
  vercel domains add <domain>         Add a domain
  vercel dns ls <domain>              List DNS records

Inspection:
  vercel inspect <url>                Show deployment details
  vercel logs <url>                   View function logs
  vercel ls                           List recent deployments

Marketplace integrations:
  vercel integration add              Install a marketplace integration
  vercel integration list             List installed integrations

Useful patterns:
  vercel link && vercel env pull      Set up a new checkout
  vercel --prod && vercel inspect     Deploy and verify";

static NOTES_SUPABASE: &str = "\
https://supabase.com/docs/reference/cli

Setup:
  supabase login                      Authenticate with Supabase
  supabase init                       Initialize a new Supabase project locally
  supabase link --project-ref <ref>   Link to a remote project
  supabase start                      Start local dev stack (requires Docker)
  supabase stop                       Stop local dev stack

Local development:
  supabase status                     Show local stack status (URLs, ports, keys)
  supabase db reset                   Reset local database to clean state
  supabase db seed                    Run seed file
  supabase inspect db                 Database size and index stats

Migrations:
  supabase migration new <name>       Create a new migration file
  supabase db diff                    Generate migration from schema changes
  supabase db push                    Push local migrations to remote database
  supabase db pull                    Pull remote schema to local migrations

Functions:
  supabase functions new <name>       Create a new edge function
  supabase functions serve            Serve functions locally with hot reload
  supabase functions deploy <name>    Deploy a function
  supabase functions deploy --all     Deploy all functions

Auth / Storage / Edge:
  supabase gen types typescript       Generate TypeScript types from schema

Useful patterns:
  supabase start && supabase status   Start and show connection details
  supabase db diff -f <name>          Generate named migration from changes
  supabase db push --dry-run          Preview migration push without executing";

static NOTES_NETLIFY: &str = "\
https://docs.netlify.com/cli/get-started/

Setup:
  netlify login                       Authenticate
  netlify init                        Initialize or link a site
  netlify link                        Link to an existing site
  netlify status                      Show current status (user, linked site)

Development:
  netlify dev                         Start local dev server (simulates Netlify edge)
  netlify dev --live                  Start with live share URL

Deploy:
  netlify deploy                      Deploy to a draft URL (preview)
  netlify deploy --prod               Deploy to production
  netlify deploy --dir=./dist         Deploy a specific directory
  netlify open                        Open site in browser

Environment variables:
  netlify env:list                    List all env vars
  netlify env:set <key> <value>       Set an env var
  netlify env:unset <key>             Remove an env var
  netlify env:import .env             Import from .env file

Functions:
  netlify functions:create <name>     Scaffold a new function
  netlify functions:serve             Serve functions locally
  netlify functions:list              List deployed functions

Build:
  netlify build                       Run build locally
  netlify build --dry                 Dry-run build

Useful patterns:
  netlify link && netlify env:list    Set up a new checkout
  netlify deploy --prod --dir=./dist  Deploy static site to production";

static NOTES_RENDER: &str = "\
https://render.com/docs/cli

Setup:
  render login                        Authenticate via browser
  render whoami                       Show current user
  render regions                      List available regions

Services:
  render services list                List all services
  render services show <id>           Show service details
  render services tail <id>           Tail service logs
  render services restart <id>        Restart a service
  render services ssh <id>            SSH to a service (if applicable)

Deploys:
  render deploys list --service <id>        List deploys
  render deploys create --service <id>      Trigger a manual deploy
  render deploys show <id>                  Show deploy details

Environment:
  render env show --service <id>            Show env vars
  render env set --service <id> KEY=VAL     Set an env var
  render env rm --service <id> KEY          Remove an env var

Blueprints (Infrastructure as Code):
  render blueprint launch             Deploy from render.yaml
  render blueprint preview            Preview blueprint changes

Useful patterns:
  render services list                Quick inventory of all services
  render services tail <id> -f        Follow logs in real time";

static NOTES_APPWRITE: &str = "\
https://appwrite.io/docs/tooling/command-line/installation

Setup:
  appwrite login                      Authenticate
  appwrite init project               Initialize a project
  appwrite init function              Initialize a function
  appwrite client setProject <id>     Set active project

Deploy:
  appwrite deploy function            Deploy functions
  appwrite deploy collection          Deploy database collections

Databases:
  appwrite databases list             List databases
  appwrite databases listDocuments    List documents in a collection

Functions:
  appwrite functions list             List functions
  appwrite functions listExecutions   List function executions
  appwrite functions createExecution  Execute a function

Users:
  appwrite users list                 List users
  appwrite users create               Create a user

Storage:
  appwrite storage listBuckets        List storage buckets
  appwrite storage listFiles          List files in a bucket

Useful patterns:
  appwrite deploy function --all      Deploy all functions at once
  appwrite init project               Start a new project locally";
