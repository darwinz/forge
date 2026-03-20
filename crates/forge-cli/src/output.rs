use std::io::Write;

use console::{Style, Term};

/// Central output handler for forge.
///
/// All terminal formatting flows through this struct. It detects whether
/// stdout is a TTY and degrades gracefully when piped or non-interactive.
pub struct Printer {
    heading_style: Style,
    success_style: Style,
    warning_style: Style,
    error_style: Style,
    dim_style: Style,
    bold_style: Style,
    interactive: bool,
}

impl Printer {
    pub fn new() -> Self {
        let interactive = Term::stdout().is_term();
        Self {
            heading_style: Style::new().bold().cyan(),
            success_style: Style::new().green(),
            warning_style: Style::new().yellow(),
            error_style: Style::new().red().bold(),
            dim_style: Style::new().dim(),
            bold_style: Style::new().bold(),
            interactive,
        }
    }

    /// Whether stdout is a TTY (interactive terminal).
    #[allow(dead_code)]
    pub fn is_interactive(&self) -> bool {
        self.interactive
    }

    // -----------------------------------------------------------------
    // Basic styled output
    // -----------------------------------------------------------------

    pub fn heading(&self, text: &str) {
        println!("{}", self.heading_style.apply_to(text));
    }

    pub fn success(&self, text: &str) {
        println!("{}", self.success_style.apply_to(text));
    }

    pub fn warning(&self, text: &str) {
        println!("{}", self.warning_style.apply_to(text));
    }

    pub fn error(&self, text: &str) {
        eprintln!("{}", self.error_style.apply_to(text));
    }

    pub fn dim(&self, text: &str) {
        println!("{}", self.dim_style.apply_to(text));
    }

    pub fn bold(&self, text: &str) {
        println!("{}", self.bold_style.apply_to(text));
    }

    pub fn print(&self, text: &str) {
        println!("{text}");
    }

    pub fn newline(&self) {
        println!();
    }

    // -----------------------------------------------------------------
    // Structured output patterns
    // -----------------------------------------------------------------

    /// Print a two-column table with bold left column.
    pub fn table(&self, rows: &[(&str, &str)], col1_width: usize) {
        for (left, right) in rows {
            println!(
                "  {:<width$}  {}",
                self.bold_style.apply_to(left),
                right,
                width = col1_width
            );
        }
    }

    /// Print a package/service status line with state-aware styling.
    pub fn package_state(&self, name: &str, state: &str) {
        let marker = match state {
            "installed" => self
                .success_style
                .apply_to(format!("[{state}]"))
                .to_string(),
            "missing" => self
                .warning_style
                .apply_to(format!("[{state}]"))
                .to_string(),
            "unavailable" => self.dim_style.apply_to(format!("[{state}]")).to_string(),
            _ => self.dim_style.apply_to(format!("[{state}]")).to_string(),
        };
        println!("  {marker}  {name}");
    }

    /// Check-mark pass line (✓).
    #[allow(dead_code)]
    pub fn check_pass(&self, label: &str) {
        let mark = self.success_style.apply_to("✓");
        println!("  {mark} {label}");
    }

    /// Check-mark fail line (✗).
    #[allow(dead_code)]
    pub fn check_fail(&self, label: &str) {
        let mark = self.error_style.apply_to("✗");
        println!("  {mark} {label}");
    }

    /// Check-mark warning line (?).
    #[allow(dead_code)]
    pub fn check_warn(&self, label: &str) {
        let mark = self.warning_style.apply_to("?");
        println!("  {mark} {label}");
    }

    /// Section header with count.
    #[allow(dead_code)]
    pub fn section(&self, title: &str, count: usize) {
        self.bold(&format!("  {title} ({count}):"));
    }

    /// Render a notes block with a title and underline.
    pub fn notes_block(&self, title: &str, content: &str) {
        self.newline();
        self.heading(&format!("Notes: {title}"));
        let underline_len = 7 + title.len(); // "Notes: " = 7 chars
        println!("{}", self.dim_style.apply_to("─".repeat(underline_len)));
        self.newline();
        self.print(content);
        self.newline();
    }

    /// Hint line — actionable suggestion.
    #[allow(dead_code)]
    pub fn hint(&self, text: &str) {
        println!("  {} {}", self.dim_style.apply_to("→"), text);
    }

    // -----------------------------------------------------------------
    // Interactive prompts
    // -----------------------------------------------------------------

    /// Confirm a destructive/important action.
    ///
    /// Returns `false` (abort) when not interactive.
    /// Uses dialoguer when interactive, falls back to raw stdin otherwise.
    pub fn confirm(&self, message: &str) -> anyhow::Result<bool> {
        if !self.interactive {
            return Ok(false);
        }

        let result = dialoguer::Confirm::new()
            .with_prompt(message)
            .default(false)
            .interact()?;
        Ok(result)
    }

    /// Select from a list of items. Returns the selected index.
    ///
    /// Fails with an error when not interactive.
    pub fn select(&self, prompt: &str, items: &[String]) -> anyhow::Result<usize> {
        if !self.interactive {
            return Err(anyhow::anyhow!(
                "Selection required but running non-interactively. Use flags to narrow your choice."
            ));
        }

        let selection = dialoguer::Select::new()
            .with_prompt(prompt)
            .items(items)
            .default(0)
            .interact()?;
        Ok(selection)
    }

    // -----------------------------------------------------------------
    // Spinner for long-running operations
    // -----------------------------------------------------------------

    /// Start a spinner on stderr (keeps stdout clean for piping).
    /// Returns a SpinnerGuard that stops on drop or explicit finish.
    #[allow(dead_code)]
    pub fn spinner(&self, message: &str) -> SpinnerGuard {
        SpinnerGuard::new(self.interactive, message)
    }
}

impl Default for Printer {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Spinner
// ---------------------------------------------------------------------------

/// A spinner that writes to stderr. Stops automatically on drop.
/// When not interactive, acts as a no-op.
pub struct SpinnerGuard {
    active: bool,
    frames: &'static [&'static str],
    frame_idx: usize,
    message: String,
    term: Term,
}

#[allow(dead_code)]
impl SpinnerGuard {
    fn new(interactive: bool, message: &str) -> Self {
        let guard = Self {
            active: interactive,
            frames: &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"],
            frame_idx: 0,
            message: message.to_string(),
            term: Term::stderr(),
        };
        if guard.active {
            let _ = write!(std::io::stderr(), "{} {}", guard.frames[0], message);
            let _ = std::io::stderr().flush();
        }
        guard
    }

    /// Advance the spinner one frame (call in a loop, or let it be).
    #[allow(dead_code)]
    pub fn tick(&mut self) {
        if !self.active {
            return;
        }
        self.frame_idx = (self.frame_idx + 1) % self.frames.len();
        let _ = self.term.clear_line();
        let _ = write!(
            std::io::stderr(),
            "\r{} {}",
            self.frames[self.frame_idx],
            self.message
        );
        let _ = std::io::stderr().flush();
    }

    /// Finish with a success message.
    pub fn finish_success(&self, message: &str) {
        if self.active {
            let _ = self.term.clear_line();
            let style = Style::new().green();
            eprintln!("\r{} {message}", style.apply_to("✓"));
        }
    }

    /// Finish with a failure message.
    pub fn finish_fail(&self, message: &str) {
        if self.active {
            let _ = self.term.clear_line();
            let style = Style::new().red().bold();
            eprintln!("\r{} {message}", style.apply_to("✗"));
        }
    }
}

impl Drop for SpinnerGuard {
    fn drop(&mut self) {
        if self.active {
            // Clear the spinner line if it wasn't explicitly finished
            let _ = self.term.clear_line();
            let _ = write!(std::io::stderr(), "\r");
            let _ = std::io::stderr().flush();
        }
    }
}
