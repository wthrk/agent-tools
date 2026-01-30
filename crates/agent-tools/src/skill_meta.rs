use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::Path;

pub use crate::fs_utils::calculate_tree_hash;

/// Metadata for an installed skill
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMeta {
    /// Source path of the skill
    pub source: String,

    /// Tree hash of the skill directory at install time
    pub tree_hash: String,

    /// When the skill was first installed
    pub installed_at: DateTime<Utc>,

    /// When the skill was last updated
    pub updated_at: DateTime<Utc>,
}

impl SkillMeta {
    /// Create new metadata for a freshly installed skill
    pub fn new(source: &Path, tree_hash: &str) -> Self {
        let now = Utc::now();
        Self {
            source: source.display().to_string(),
            tree_hash: tree_hash.to_string(),
            installed_at: now,
            updated_at: now,
        }
    }

    /// Load metadata from a .skill-meta.yaml file
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path).context("Failed to read .skill-meta.yaml")?;
        serde_yaml::from_str(&content).context("Failed to parse .skill-meta.yaml")
    }

    /// Save metadata to a .skill-meta.yaml file
    pub fn save(&self, path: &Path) -> Result<()> {
        let content = serde_yaml::to_string(self).context("Failed to serialize metadata")?;
        std::fs::write(path, content).context("Failed to write .skill-meta.yaml")
    }
}
