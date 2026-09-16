//! The two project-level settings an SCM process has to agree with the runs
//! it reads: where a model's run output lands, and which comment dialect
//! names the parameters.

use anyhow::Result;
use std::path::Path;

use config::{CommentType, Config, NonmemConfig, find_config_dir_from};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct RunSettings {
    pub output_dir: Option<String>,
    pub comment_type: Option<CommentType>,
}

impl RunSettings {
    pub fn from_config(config: &NonmemConfig) -> Self {
        Self {
            output_dir: config.output_dir.clone(),
            comment_type: config.comments.r#type,
        }
    }

    pub fn discover_from(dir: impl AsRef<Path>) -> Result<Self> {
        let dir = dir.as_ref();
        let config_dir = find_config_dir_from(dir)?.ok_or_else(|| {
            anyhow::anyhow!(
                "no {} found in '{}' or any directory above it",
                config::CONFIG_FILENAME,
                dir.display()
            )
        })?;
        Self::load(config_dir)
    }

    fn load(config_dir: std::path::PathBuf) -> Result<Self> {
        let config = Config::load(config_dir.join(config::CONFIG_FILENAME))?;
        Ok(config
            .nonmem
            .as_ref()
            .map(Self::from_config)
            .unwrap_or_default())
    }
}
