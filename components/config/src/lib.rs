mod nonmem;
mod output_dir_templating;

pub use output_dir_templating::render_output_dir_template;

use std::path::{Component, Path, PathBuf};

use anyhow::{Result, anyhow};
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

/// Convert an absolute path to be relative to the pharos config directory.
/// Returns the original path if no config directory is found.
pub fn to_config_relative(path: impl AsRef<Path>) -> Result<PathBuf> {
    let path = path.as_ref();
    let config_dir = find_config_dir()?.ok_or_else(|| anyhow!("Failed to find config dir"))?;

    let rel = make_relative_path(&config_dir, path);

    Ok(rel)
}

fn make_relative_path(base: &Path, target: &Path) -> PathBuf {
    let base_components: Vec<Component<'_>> = base.components().collect();
    let target_components: Vec<Component<'_>> = target.components().collect();

    if base_components.first() != target_components.first() {
        return target.to_path_buf();
    }

    let mut idx = 0;
    let max = base_components.len().min(target_components.len());
    while idx < max && base_components[idx] == target_components[idx] {
        idx += 1;
    }

    let mut rel = PathBuf::new();
    for _ in idx..base_components.len() {
        rel.push("..");
    }
    for comp in target_components.iter().skip(idx) {
        rel.push(comp.as_os_str());
    }

    rel
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
