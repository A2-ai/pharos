mod nonmem;
mod output_dir_templating;

pub use output_dir_templating::render_output_dir_template;

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use anyhow::{Result, anyhow};
use fs_err as fs;
use serde::{Deserialize, Serialize};

pub use crate::nonmem::{CommentType, CommentsConfig, NonmemConfig};

pub const CONFIG_FILENAME: &str = "pharos.toml";

/// Process-wide override for the project root, set once at startup when
/// `--config-file` is supplied. Consulted by [`find_config_dir`].
static CONFIG_DIR_OVERRIDE: OnceLock<PathBuf> = OnceLock::new();

/// Set the project root explicitly. Intended to be called once from `main`
/// after CLI parsing when `--config-file` is given. Subsequent calls are
/// ignored.
pub fn set_config_dir(dir: PathBuf) {
    let _ = CONFIG_DIR_OVERRIDE.set(dir);
}

/// Find where the root dir is (eg where the config file).
/// If we can't find it and we reached a .git folder/no more parent folder, this returns None.
pub fn find_config_dir() -> Result<Option<PathBuf>> {
    if let Some(dir) = CONFIG_DIR_OVERRIDE.get() {
        return Ok(Some(dir.clone()));
    }

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

/// Convert an absolute path to a stable, project-relative identifier — a
/// string of path components joined by forward slashes (e.g.
/// `"model/nonmem/struct/1001.mod"`). The forward-slash form makes
/// identifiers stable across platforms; metadata written on one OS reads
/// correctly on another. Errors if the path is outside the config directory.
pub fn to_config_relative(path: impl AsRef<Path>) -> Result<String> {
    let path = path.as_ref();
    let config_dir =
        fs::canonicalize(find_config_dir()?.ok_or_else(|| anyhow!("Failed to find config dir"))?)?;
    let rel = path.strip_prefix(&config_dir).map_err(|_| {
        anyhow!(
            "'{}' is outside the project root '{}'",
            path.display(),
            config_dir.display()
        )
    })?;
    Ok(rel
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/"))
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
