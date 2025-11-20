use std::io::Write;
use std::path::Path;

use anyhow::Result;
use fs_err as fs;
use serde::Serialize;

mod env;
mod time;

pub use env::get_masked_env_vars;
pub use time::get_utc_now;

pub fn write_json_to_file<T: Serialize, P: AsRef<Path>>(data: &T, path: P) -> Result<()> {
    let json_string = serde_json::to_string_pretty(data)?;
    let mut file = fs::File::create(path.as_ref())?;
    file.write_all(json_string.as_bytes())?;
    file.write_all(b"\n")?;
    file.flush()?;

    Ok(())
}
