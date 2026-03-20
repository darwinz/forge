//! Terminal rendering engine for forge.
//!
//! Interprets text with lightweight Markdown-like patterns and renders it
//! with terminal styling. Falls back to plain text when not interactive.
//!
//! Recognized patterns:
//! - Lines starting with `# ` → heading (bold cyan, underlined)
//! - Lines starting with `## ` → subheading (bold)
//! - Lines starting with `- ` or `* ` → bullet list items
//! - Lines starting with `  ` (2+ spaces) that look like commands → code (green)
//! - Lines starting with `> ` → callout/tip (yellow, with bar)
//! - Lines starting with `http://` or `https://` → link (dim, underlined)
//! - Lines that are all `-` or `=` → divider
//! - Lines ending with `:` → section label (bold)
//! - `**text**` inline → bold
//! - `` `text` `` inline → code highlight
//! - Everything else → plain text

use console::Style;

/// Render styled content to a String.
/// When `styled` is true, applies terminal formatting.
/// When false, returns plain text (for piping).
pub fn render(content: &str, styled: bool) -> String {
    if !styled {
        return content.to_string();
    }

    let mut output = Vec::new();

    for line in content.lines() {
        output.push(render_line(line));
    }

    output.join("\n")
}

fn render_line(line: &str) -> String {
    let trimmed = line.trim();

    // Empty line
    if trimmed.is_empty() {
        return String::new();
    }

    // Heading: # text
    if let Some(text) = trimmed.strip_prefix("# ") {
        let style = Style::new().bold().cyan();
        let underline = Style::new().dim();
        return format!(
            "{}\n{}",
            style.apply_to(text),
            underline.apply_to("─".repeat(text.len()))
        );
    }

    // Subheading: ## text
    if let Some(text) = trimmed.strip_prefix("## ") {
        let style = Style::new().bold();
        return format!("{}", style.apply_to(text));
    }

    // Callout/tip: > text
    if let Some(text) = trimmed.strip_prefix("> ") {
        let bar = Style::new().yellow();
        let body = Style::new().yellow();
        return format!("  {} {}", bar.apply_to("│"), body.apply_to(text));
    }

    // Divider: --- or === or ───
    if trimmed.len() >= 3
        && (trimmed.chars().all(|c| c == '-')
            || trimmed.chars().all(|c| c == '=')
            || trimmed.chars().all(|c| c == '─'))
    {
        let style = Style::new().dim();
        return format!("{}", style.apply_to("─".repeat(40)));
    }

    // URL line (starts with http)
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        let style = Style::new().dim().underlined();
        return format!("  {}", style.apply_to(trimmed));
    }

    // Bullet list: - item or * item
    if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
        let text = &trimmed[2..];
        let bullet = Style::new().cyan();
        return format!("  {} {}", bullet.apply_to("•"), render_inline(text));
    }

    // Command line: indented with 2+ spaces and looks like a command
    if line.starts_with("  ") && !line.starts_with("    ") && looks_like_command(trimmed) {
        let cmd_style = Style::new().green();
        // Split command from description at first double-space or dash separator
        if let Some((cmd, desc)) = split_command_desc(trimmed) {
            let dim = Style::new().dim();
            return format!("  {}  {}", cmd_style.apply_to(cmd), dim.apply_to(desc));
        }
        return format!("  {}", cmd_style.apply_to(trimmed));
    }

    // Section label: line ending with `:` (and not indented)
    if trimmed.ends_with(':') && !line.starts_with(' ') && trimmed.len() > 1 {
        let style = Style::new().bold();
        return format!("\n{}", style.apply_to(trimmed));
    }

    // Section label: indented line ending with `:`
    if trimmed.ends_with(':') && line.starts_with(' ') && trimmed.len() > 1 {
        let style = Style::new().bold();
        return format!("  {}", style.apply_to(trimmed));
    }

    // Comment line: starts with #
    if trimmed.starts_with('#') && !trimmed.starts_with("##") {
        let style = Style::new().dim();
        return format!("  {}", style.apply_to(trimmed));
    }

    // Default: render inline formatting on the raw line
    render_inline(line).to_string()
}

/// Render inline formatting: **bold**, `code`, and plain text.
#[allow(clippy::while_let_on_iterator)]
fn render_inline(text: &str) -> String {
    let mut result = String::new();
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        // **bold**
        if ch == '*' && chars.peek() == Some(&'*') {
            chars.next(); // consume second *
            let mut bold_text = String::new();
            while let Some(c) = chars.next() {
                if c == '*' && chars.peek() == Some(&'*') {
                    chars.next();
                    break;
                }
                bold_text.push(c);
            }
            let style = Style::new().bold();
            result.push_str(&style.apply_to(&bold_text).to_string());
        }
        // `code`
        else if ch == '`' {
            let mut code_text = String::new();
            while let Some(c) = chars.next() {
                if c == '`' {
                    break;
                }
                code_text.push(c);
            }
            let style = Style::new().green();
            result.push_str(&style.apply_to(&code_text).to_string());
        } else {
            result.push(ch);
        }
    }

    result
}

/// Check if a trimmed line looks like a CLI command.
fn looks_like_command(line: &str) -> bool {
    if line.is_empty() {
        return false;
    }
    let first_word = line.split_whitespace().next().unwrap_or("");
    // Commands typically start with a known tool name or contain special chars
    first_word.contains('/')
        || first_word.contains('.')
        || first_word.contains('=')
        || first_word.starts_with('-')
        || is_known_command(first_word)
}

fn is_known_command(word: &str) -> bool {
    matches!(
        word,
        "forge"
            | "git"
            | "docker"
            | "kubectl"
            | "helm"
            | "terraform"
            | "aws"
            | "gcloud"
            | "npm"
            | "cargo"
            | "go"
            | "python3"
            | "pip"
            | "uv"
            | "brew"
            | "curl"
            | "ssh"
            | "scp"
            | "screen"
            | "tmux"
            | "packer"
            | "blackbox_initialize"
            | "blackbox_addadmin"
            | "blackbox_register_new_file"
            | "blackbox_edit_start"
            | "blackbox_edit_end"
            | "blackbox_diff"
            | "blackbox_decrypt_all_files"
            | "blackbox_shred_all_files"
            | "blackbox_postdeploy"
            | "gpg"
            | "jo"
            | "http"
            | "https"
            | "dust"
            | "fd"
            | "sd"
            | "procs"
            | "btm"
            | "topgrade"
            | "tokei"
            | "eva"
            | "op"
            | "claude"
            | "codex"
            | "vercel"
            | "supabase"
            | "netlify"
            | "render"
            | "appwrite"
    )
}

/// Split a command line into (command, description) at a separator.
/// Looks for ` - `, ` — `, or a long gap (3+ spaces).
fn split_command_desc(line: &str) -> Option<(&str, &str)> {
    // Try " - " separator
    if let Some(pos) = line.find(" - ") {
        return Some((line[..pos].trim(), line[pos + 3..].trim()));
    }
    // Try " — " separator
    if let Some(pos) = line.find(" — ") {
        let dash_len = " — ".len();
        return Some((line[..pos].trim(), line[pos + dash_len..].trim()));
    }
    // Try long whitespace gap (3+ spaces in the middle)
    let trimmed = line.trim();
    if let Some(pos) = trimmed.find("   ") {
        let cmd = trimmed[..pos].trim();
        let desc = trimmed[pos..].trim();
        if !cmd.is_empty() && !desc.is_empty() {
            return Some((cmd, desc));
        }
    }
    None
}

/// Render a notes page with a title header.
pub fn render_notes_page(title: &str, content: &str, styled: bool) -> String {
    let mut output = String::new();

    if styled {
        let title_style = Style::new().bold().cyan();
        let underline_style = Style::new().dim();
        let title_text = format!("Notes: {title}");
        output.push('\n');
        output.push_str(&title_style.apply_to(&title_text).to_string());
        output.push('\n');
        output.push_str(
            &underline_style
                .apply_to("─".repeat(title_text.len()))
                .to_string(),
        );
        output.push('\n');
        output.push('\n');
        output.push_str(&render(content, true));
        output.push('\n');
    } else {
        output.push_str(&format!("Notes: {title}\n"));
        output.push_str(&format!("{}\n", "─".repeat(7 + title.len())));
        output.push('\n');
        output.push_str(content);
        output.push('\n');
    }

    output
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_plain_passthrough() {
        let input = "hello world\nline two";
        let output = render(input, false);
        assert_eq!(output, input);
    }

    #[test]
    fn test_render_heading() {
        let output = render("# My Heading", true);
        assert!(output.contains("My Heading"));
        assert!(output.contains("─")); // underline
    }

    #[test]
    fn test_render_subheading() {
        let output = render("## Sub", true);
        assert!(output.contains("Sub"));
    }

    #[test]
    fn test_render_url() {
        let output = render("https://example.com", true);
        assert!(output.contains("example.com"));
    }

    #[test]
    fn test_render_bullet() {
        let output = render("- item one", true);
        assert!(output.contains("•"));
        assert!(output.contains("item one"));
    }

    #[test]
    fn test_render_callout() {
        let output = render("> this is a tip", true);
        assert!(output.contains("│"));
        assert!(output.contains("this is a tip"));
    }

    #[test]
    fn test_render_divider() {
        let output = render("---", true);
        assert!(output.contains("─"));
    }

    #[test]
    fn test_render_command_line() {
        let output = render("  kubectl get pods", true);
        assert!(output.contains("kubectl"));
    }

    #[test]
    fn test_render_command_with_desc() {
        let output = render("  forge notes              List all notes", true);
        assert!(output.contains("forge notes"));
    }

    #[test]
    fn test_render_section_label() {
        let output = render("Commands:", true);
        assert!(output.contains("Commands:"));
    }

    #[test]
    fn test_render_inline_bold() {
        let output = render_inline("this is **bold** text");
        assert!(output.contains("bold"));
    }

    #[test]
    fn test_render_inline_code() {
        let output = render_inline("run `forge notes`");
        assert!(output.contains("forge notes"));
    }

    #[test]
    fn test_split_command_desc_dash() {
        let (cmd, desc) = split_command_desc("forge notes - List topics").unwrap();
        assert_eq!(cmd, "forge notes");
        assert_eq!(desc, "List topics");
    }

    #[test]
    fn test_split_command_desc_spaces() {
        let (cmd, desc) = split_command_desc("forge notes              List topics").unwrap();
        assert_eq!(cmd, "forge notes");
        assert_eq!(desc, "List topics");
    }

    #[test]
    fn test_split_command_desc_none() {
        assert!(split_command_desc("forge notes").is_none());
    }

    #[test]
    fn test_notes_page_styled() {
        let output = render_notes_page("k8s", "  kubectl get pods", true);
        assert!(output.contains("Notes: k8s"));
        assert!(output.contains("─"));
        assert!(output.contains("kubectl"));
    }

    #[test]
    fn test_notes_page_plain() {
        let output = render_notes_page("k8s", "  kubectl get pods", false);
        assert!(output.contains("Notes: k8s"));
        assert!(output.contains("kubectl get pods"));
        // Plain mode should not have ANSI escape sequences
        assert!(!output.contains("\x1b["));
    }

    #[test]
    fn test_looks_like_command() {
        assert!(looks_like_command("kubectl get pods"));
        assert!(looks_like_command("forge notes"));
        assert!(looks_like_command("--flag value"));
        assert!(!looks_like_command(""));
        assert!(!looks_like_command("plain text here"));
    }
}
