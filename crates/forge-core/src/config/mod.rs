pub mod schema;

use std::path::{Path, PathBuf};

use crate::error::{ForgeError, ForgeResult};

pub use schema::ForgeConfig;

/// Resolve the forge config directory (~/.forge)
pub fn forge_dir() -> PathBuf {
    dirs_home().join(".forge")
}

/// Resolve home directory
fn dirs_home() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"))
}

/// Load config with 3-layer merge: defaults -> user config -> overrides
pub fn load_config(config_path: Option<&Path>) -> ForgeResult<ForgeConfig> {
    // Layer 1: built-in defaults
    let mut config = ForgeConfig::default();

    // Layer 2: user config file
    let user_config_path = config_path
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| forge_dir().join("config.toml"));

    if user_config_path.exists() {
        let content =
            std::fs::read_to_string(&user_config_path).map_err(|e| ForgeError::FileRead {
                path: user_config_path.clone(),
                source: e,
            })?;
        let user: ForgeConfig = toml::from_str(&content).map_err(|e| ForgeError::TomlParse {
            path: user_config_path,
            source: e,
        })?;
        config.merge(user);
    }

    // Layer 3: CLI overrides are applied by the caller after loading

    Ok(config)
}
