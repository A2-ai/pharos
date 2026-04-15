use miniextendr_api::miniextendr;

use std::path::PathBuf;

// pharos config crate
use config::{CONFIG_FILENAME, find_config_dir as pharos_find_config_dir};

/// Find the pharos config directory, returning anyhow::Result
pub fn find_config_dir() -> anyhow::Result<Option<PathBuf>> {
    pharos_find_config_dir().map_err(|e| anyhow::anyhow!("Failed to find config dir: {e:?}"))
}

/// Suppress panic messages from pharos
///
/// @return NULL
/// @export
#[miniextendr]
pub fn set_panic_message() {
    std::panic::set_hook(Box::new(|_| {}));
}

/// Find the pharos config file path
///
/// @return path to pharos.toml, or a message if not found
/// @export
#[miniextendr]
pub fn find_pharos_config_file() -> Result<String, anyhow::Error> {
    let config_dir = find_config_dir()?;

    match config_dir {
        Some(d) => Ok(d.join(CONFIG_FILENAME).to_string_lossy().to_string()),
        None => Ok(
            "No pharos.toml config file found. Please call hyperion::init() to create one"
                .to_string(),
        ),
    }
}
