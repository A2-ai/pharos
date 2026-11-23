use std::path::Path;

use anyhow::Result;
use fs_err as fs;
use serde::Serialize;

mod env;
mod path;
mod time;

pub use env::get_masked_env_vars;
pub use path::normalize_path;
pub use time::{format_duration, get_utc_now, seconds_between};

pub fn write_json_to_file<T: Serialize, P: AsRef<Path>>(data: &T, path: P) -> Result<()> {
    let path = path.as_ref();
    let json_string = serde_json::to_string_pretty(data)? + "\n";
    // Written beside the file and renamed over it, so nobody reads a half-written one
    let staging = path.with_extension(format!("{}.tmp", std::process::id()));
    fs::write(&staging, json_string)?;
    fs::rename(&staging, path)?;
    Ok(())
}
