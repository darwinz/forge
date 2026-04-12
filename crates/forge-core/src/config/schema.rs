use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ForgeConfig {
    #[serde(default)]
    pub general: GeneralConfig,
    #[serde(default)]
    pub shell: ShellConfig,
    #[serde(default)]
    pub bootstrap: BootstrapConfig,
    #[serde(default)]
    pub skills: SkillsConfig,
    #[serde(default)]
    pub notes: NotesConfig,
}

impl ForgeConfig {
    /// Merge another config into this one (non-default values override)
    pub fn merge(&mut self, other: ForgeConfig) {
        self.general.log_level = other.general.log_level;
        self.shell.shell_type = other.shell.shell_type;
        if !other.bootstrap.bundle_dir.is_empty() {
            self.bootstrap.bundle_dir = other.bootstrap.bundle_dir;
        }
        if !other.skills.user_dir.is_empty() {
            self.skills.user_dir = other.skills.user_dir;
        }
        if !other.notes.user_dir.is_empty() {
            self.notes.user_dir = other.notes.user_dir;
        }
        if !other.notes.repo_dir.is_empty() {
            self.notes.repo_dir = other.notes.repo_dir;
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            log_level: default_log_level(),
        }
    }
}

fn default_log_level() -> String {
    "warn".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellConfig {
    #[serde(default = "default_shell_type", rename = "type")]
    pub shell_type: String,
}

impl Default for ShellConfig {
    fn default() -> Self {
        Self {
            shell_type: default_shell_type(),
        }
    }
}

fn default_shell_type() -> String {
    "zsh".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapConfig {
    #[serde(default = "default_bundle_dir")]
    pub bundle_dir: String,
    #[serde(default = "default_default_profile")]
    pub default_profile: String,
}

impl Default for BootstrapConfig {
    fn default() -> Self {
        Self {
            bundle_dir: default_bundle_dir(),
            default_profile: default_default_profile(),
        }
    }
}

fn default_bundle_dir() -> String {
    "config/bundles".to_string()
}

fn default_default_profile() -> String {
    "config/default-profile.toml".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillsConfig {
    #[serde(default = "default_skills_user_dir")]
    pub user_dir: String,
    #[serde(default = "default_skills_repo_dir")]
    pub repo_dir: String,
    #[serde(default = "default_audit_log")]
    pub audit_log: String,
}

impl Default for SkillsConfig {
    fn default() -> Self {
        Self {
            user_dir: default_skills_user_dir(),
            repo_dir: default_skills_repo_dir(),
            audit_log: default_audit_log(),
        }
    }
}

fn default_skills_user_dir() -> String {
    "~/.forge/skills".to_string()
}

fn default_skills_repo_dir() -> String {
    ".forge/skills".to_string()
}

fn default_audit_log() -> String {
    "~/.forge/audit/skills.jsonl".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotesConfig {
    #[serde(default = "default_notes_user_dir")]
    pub user_dir: String,
    #[serde(default = "default_notes_repo_dir")]
    pub repo_dir: String,
}

impl Default for NotesConfig {
    fn default() -> Self {
        Self {
            user_dir: default_notes_user_dir(),
            repo_dir: default_notes_repo_dir(),
        }
    }
}

fn default_notes_user_dir() -> String {
    "~/.forge/notes".to_string()
}

fn default_notes_repo_dir() -> String {
    ".forge/notes".to_string()
}
