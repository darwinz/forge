use console::Style;

pub struct Printer {
    heading: Style,
    success: Style,
    warning: Style,
    error: Style,
    dim: Style,
    bold: Style,
}

impl Printer {
    pub fn new() -> Self {
        Self {
            heading: Style::new().bold().cyan(),
            success: Style::new().green(),
            warning: Style::new().yellow(),
            error: Style::new().red().bold(),
            dim: Style::new().dim(),
            bold: Style::new().bold(),
        }
    }

    pub fn heading(&self, text: &str) {
        println!("{}", self.heading.apply_to(text));
    }

    pub fn success(&self, text: &str) {
        println!("{}", self.success.apply_to(text));
    }

    #[allow(dead_code)]
    pub fn warning(&self, text: &str) {
        println!("{}", self.warning.apply_to(text));
    }

    pub fn error(&self, text: &str) {
        eprintln!("{}", self.error.apply_to(text));
    }

    pub fn dim(&self, text: &str) {
        println!("{}", self.dim.apply_to(text));
    }

    pub fn bold(&self, text: &str) {
        println!("{}", self.bold.apply_to(text));
    }

    pub fn print(&self, text: &str) {
        println!("{text}");
    }

    pub fn newline(&self) {
        println!();
    }

    /// Print a two-column table
    pub fn table(&self, rows: &[(&str, &str)], col1_width: usize) {
        for (left, right) in rows {
            println!(
                "  {:<width$}  {}",
                self.bold.apply_to(left),
                right,
                width = col1_width
            );
        }
    }

    /// Print a package status line with state-aware styling.
    pub fn package_state(&self, name: &str, state: &str) {
        let marker = match state {
            "installed" => self.success.apply_to(format!("[{state}]")).to_string(),
            "missing" => self.warning.apply_to(format!("[{state}]")).to_string(),
            "unavailable" => self.dim.apply_to(format!("[{state}]")).to_string(),
            _ => self.dim.apply_to(format!("[{state}]")).to_string(), // "unknown"
        };
        println!("  {marker}  {name}");
    }
}

impl Default for Printer {
    fn default() -> Self {
        Self::new()
    }
}
