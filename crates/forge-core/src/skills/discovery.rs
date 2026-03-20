use super::metadata::{DiscoveredSkill, SkillManifest, SkillSource};
use crate::error::{ForgeError, ForgeResult};
use std::path::{Path, PathBuf};

/// Discover skills from all search paths
pub fn discover_skills(
    user_dir: &Path,
    repo_dir: Option<&Path>,
) -> ForgeResult<Vec<DiscoveredSkill>> {
    let mut skills = Vec::new();

    // User-global skills
    if user_dir.exists() {
        discover_in_dir(user_dir, SkillSource::UserGlobal, &mut skills)?;
    }

    // Repo-scoped skills (higher priority)
    if let Some(dir) = repo_dir {
        if dir.exists() {
            discover_in_dir(dir, SkillSource::RepoScoped, &mut skills)?;
        }
    }

    Ok(skills)
}

fn discover_in_dir(
    dir: &Path,
    source: SkillSource,
    skills: &mut Vec<DiscoveredSkill>,
) -> ForgeResult<()> {
    let entries = std::fs::read_dir(dir).map_err(|e| ForgeError::FileRead {
        path: dir.to_path_buf(),
        source: e,
    })?;

    for entry in entries.flatten() {
        let skill_toml = entry.path().join("skill.toml");
        if skill_toml.exists() {
            match load_skill_manifest(&skill_toml) {
                Ok(manifest) => {
                    skills.push(DiscoveredSkill {
                        manifest,
                        path: entry.path(),
                        source: source.clone(),
                    });
                }
                Err(e) => {
                    tracing::warn!("skipping skill at {:?}: {}", entry.path(), e);
                }
            }
        }
    }

    Ok(())
}

fn load_skill_manifest(path: &Path) -> ForgeResult<SkillManifest> {
    let content = std::fs::read_to_string(path).map_err(|e| ForgeError::FileRead {
        path: path.to_path_buf(),
        source: e,
    })?;

    toml::from_str(&content).map_err(|e| ForgeError::TomlParse {
        path: path.to_path_buf(),
        source: e,
    })
}

/// Resolve the user skills directory from config (handles ~ expansion)
pub fn resolve_skills_dir(dir: &str) -> PathBuf {
    if dir.starts_with('~') {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(dir.replacen('~', &home, 1));
        }
    }
    PathBuf::from(dir)
}
