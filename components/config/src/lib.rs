mod nonmem;
mod output_dir_templating;

pub use output_dir_templating::render_output_dir_template;

use std::path::{Path, PathBuf};

use anyhow::Result;
use fs_err as fs;
use serde::{Deserialize, Serialize};

pub use crate::nonmem::{CommentType, CommentsConfig, NonmemConfig};

pub const CONFIG_FILENAME: &str = "pharos.toml";

/// Find where the root dir is (eg where the config file).
/// If we can't find it and we reached a .git folder/no more parent folder, this returns None.
pub fn find_config_dir() -> Result<Option<PathBuf>> {
    let mut current = std::env::current_dir()?;

    loop {
        if current.join(CONFIG_FILENAME).exists() {
            return Ok(Some(current));
        }

        if current.join(".git").is_dir() {
            break;
        }

        if let Some(parent) = current.parent() {
            current = parent.to_path_buf();
        } else {
            break;
        }
    }

    Ok(None)
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub nonmem: Option<NonmemConfig>,
}

impl Config {
    pub fn new_nonmem() -> Result<Self> {
        Ok(Self {
            nonmem: Some(NonmemConfig::new()?),
        })
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let content = fs::read_to_string(path)?;
        Ok(toml::from_str(&content)?)
    }
}
