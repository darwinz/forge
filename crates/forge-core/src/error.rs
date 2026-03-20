use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum ForgeError {
    #[error("configuration error: {0}")]
    Config(String),

    #[error("failed to read file {path}: {source}")]
    FileRead {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("failed to parse TOML in {path}: {source}")]
    TomlParse {
        path: PathBuf,
        source: toml::de::Error,
    },

    #[error("bundle not found: {0}")]
    BundleNotFound(String),

    #[error("skill not found: {0}")]
    SkillNotFound(String),

    #[error("skill validation error in {skill}: {message}")]
    SkillValidation { skill: String, message: String },

    #[error("command failed: {0}")]
    CommandFailed(String),

    #[error("unsupported platform: {0}")]
    UnsupportedPlatform(String),

    #[error("notes topic not found: {0}")]
    NotesTopicNotFound(String),

    #[error("{0}")]
    Other(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub type ForgeResult<T> = Result<T, ForgeError>;
