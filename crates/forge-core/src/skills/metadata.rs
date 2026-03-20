use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

/// Skill manifest loaded from skill.toml
#[derive(Debug, Clone, Deserialize)]
pub struct SkillManifest {
    pub skill: SkillInfo,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SkillInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub metadata: Option<SkillMetadata>,
    #[serde(default)]
    pub inputs: Option<HashMap<String, InputDef>>,
    #[serde(default)]
    pub permissions: Option<SkillPermissions>,
    #[serde(default)]
    pub execution: Option<SkillExecution>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SkillMetadata {
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub min_forge_version: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct InputDef {
    #[serde(rename = "type")]
    pub input_type: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SkillPermissions {
    #[serde(default)]
    pub read_files: bool,
    #[serde(default)]
    pub write_files: bool,
    #[serde(default)]
    pub network: bool,
    #[serde(default)]
    pub execute_commands: bool,
    #[serde(default)]
    pub max_file_size_bytes: Option<u64>,
    #[serde(default)]
    pub timeout_seconds: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SkillExecution {
    #[serde(rename = "type")]
    pub exec_type: String,
    pub entrypoint: String,
    #[serde(default)]
    pub interpreter: Option<String>,
}

/// A discovered skill with its filesystem location
#[derive(Debug, Clone)]
pub struct DiscoveredSkill {
    pub manifest: SkillManifest,
    pub path: PathBuf,
    pub source: SkillSource,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SkillSource {
    BuiltIn,
    UserGlobal,
    RepoScoped,
}

impl std::fmt::Display for SkillSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SkillSource::BuiltIn => write!(f, "built-in"),
            SkillSource::UserGlobal => write!(f, "user"),
            SkillSource::RepoScoped => write!(f, "repo"),
        }
    }
}
