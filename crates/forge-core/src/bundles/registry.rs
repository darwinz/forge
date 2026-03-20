use std::path::{Path, PathBuf};
use serde::Deserialize;
use crate::error::{ForgeError, ForgeResult};

/// A bundle manifest loaded from TOML
#[derive(Debug, Clone, Deserialize)]
pub struct BundleManifest {
    pub bundle: BundleInfo,
    #[serde(default)]
    pub brew: Option<BrewSection>,
    #[serde(default)]
    pub npm: Option<NpmSection>,
    #[serde(default)]
    pub pnpm: Option<PnpmSection>,
    #[serde(default)]
    pub bun: Option<BunSection>,
    #[serde(default)]
    pub go: Option<GoSection>,
    #[serde(default)]
    pub gem: Option<GemSection>,
    #[serde(default)]
    pub pipx: Option<PipxSection>,
    #[serde(default)]
    pub uv_tool: Option<UvToolSection>,
    #[serde(default)]
    pub composer: Option<ComposerSection>,
    #[serde(default)]
    pub git_aliases: Option<GitAliasesSection>,
    #[serde(default)]
    pub manual: Option<Vec<ManualEntry>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BundleInfo {
    pub name: String,
    pub description: String,
    pub tier: String,
    #[serde(default)]
    pub required: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BrewSection {
    #[serde(default)]
    pub formulae: Vec<String>,
    #[serde(default)]
    pub casks: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NpmSection {
    #[serde(default)]
    pub packages: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GoSection {
    #[serde(default)]
    pub tools: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GemSection {
    #[serde(default)]
    pub packages: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PipxSection {
    #[serde(default)]
    pub packages: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UvToolSection {
    #[serde(default)]
    pub tools: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PnpmSection {
    #[serde(default)]
    pub packages: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BunSection {
    #[serde(default)]
    pub packages: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ComposerSection {
    #[serde(default)]
    pub packages: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GitAliasesSection {
    pub file: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ManualEntry {
    pub name: String,
    pub check_command: String,
    pub install_instructions: String,
    #[serde(default)]
    pub notes: Option<String>,
}

/// Registry of all available bundles
pub struct BundleRegistry {
    bundles: Vec<BundleManifest>,
}

impl BundleRegistry {
    /// Load all bundle manifests from a directory
    pub fn load(bundle_dir: &Path) -> ForgeResult<Self> {
        let mut bundles = Vec::new();

        if !bundle_dir.exists() {
            return Ok(Self { bundles });
        }

        let mut entries: Vec<PathBuf> = std::fs::read_dir(bundle_dir)
            .map_err(|e| ForgeError::FileRead {
                path: bundle_dir.to_path_buf(),
                source: e,
            })?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|ext| ext == "toml"))
            .collect();

        entries.sort();

        for path in entries {
            let content = std::fs::read_to_string(&path).map_err(|e| ForgeError::FileRead {
                path: path.clone(),
                source: e,
            })?;
            let manifest: BundleManifest = toml::from_str(&content).map_err(|e| {
                ForgeError::TomlParse {
                    path: path.clone(),
                    source: e,
                }
            })?;
            bundles.push(manifest);
        }

        Ok(Self { bundles })
    }

    /// Get all bundles
    pub fn all(&self) -> &[BundleManifest] {
        &self.bundles
    }

    /// Find a bundle by name
    pub fn find(&self, name: &str) -> Option<&BundleManifest> {
        self.bundles.iter().find(|b| b.bundle.name == name)
    }

    /// List bundles grouped by tier
    pub fn list_by_tier(&self) -> Vec<(&str, Vec<&BundleManifest>)> {
        let tier_order = ["core", "role", "experimental", "ai-tools"];
        let mut result = Vec::new();

        for tier in &tier_order {
            let tier_bundles: Vec<&BundleManifest> = self.bundles.iter()
                .filter(|b| b.bundle.tier == *tier)
                .collect();
            if !tier_bundles.is_empty() {
                result.push((*tier, tier_bundles));
            }
        }

        result
    }
}
