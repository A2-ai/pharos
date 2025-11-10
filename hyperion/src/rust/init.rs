use extendr_api::prelude::*;
use fs_err as fs;
use std::io::Write;
use std::path::Path;

// pharos config crate
use config::{CONFIG_FILENAME, Config};

/// Initializes pharos
///
/// @param config_path path to where pharos.toml is saved (should be colocated to where pharos is
/// run from)
///
/// @return nothing
/// @export
///
/// @examples \dontrun{
/// init("model/nonmem/submission-log/pharos.toml")
/// }
#[extendr]
fn init(config_path: &str) -> Result<()> {
    let path = Path::new(config_path);

    let config_path = if path.is_dir() {
        path.join(CONFIG_FILENAME)
    } else if path.file_name() == Some(std::ffi::OsStr::new(CONFIG_FILENAME)) {
        path.to_path_buf()
    } else {
        path.with_file_name(CONFIG_FILENAME)
    };

    if config_path.exists() {
        return Err(Error::Other("nonmem config file already exists".into()));
    }

    // Create parent directories if they don't exist
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent).map_err(|e| Error::Other(format!("{e}")))?;
    }

    let mut config_file =
        fs::File::create(&config_path).map_err(|e| Error::Other(format!("{e}")))?;

    let nonmem_config = Config::new_nonmem()
        .map_err(|e| Error::Other(format!("Failed to create nonmem config: {e}")))?;

    let config = toml::to_string_pretty(&nonmem_config).map_err(|e| Error::Other(e.to_string()))?;

    config_file
        .write_all(config.as_bytes())
        .map_err(|x| Error::Other(x.to_string()))?;

    Ok(())
}

extendr_module! {
    mod init;
    fn init;
}
