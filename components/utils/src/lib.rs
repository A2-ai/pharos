use std::io::Write;
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

/// Write `data` to `path` as JSON, whole or not at all: the text goes to a
/// file beside `path` that is then renamed over it, so a reader never meets
/// a half-written file and a writer killed mid-way leaves the old one.
pub fn write_json_to_file<T: Serialize, P: AsRef<Path>>(data: &T, path: P) -> Result<()> {
    let path = path.as_ref();
    let json_string = serde_json::to_string_pretty(data)?;
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(format!(".{}.tmp", std::process::id()));
    let tmp = path.with_file_name(name);
    let written = (|| -> Result<()> {
        let mut file = fs::File::create(&tmp)?;
        file.write_all(json_string.as_bytes())?;
        file.write_all(b"\n")?;
        file.flush()?;
        drop(file);
        fs::rename(&tmp, path)?;
        Ok(())
    })();
    if written.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    written
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_json_file_is_replaced_whole_and_leaves_nothing_beside_it() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.json");
        write_json_to_file(&vec![1, 2, 3], &path).unwrap();
        write_json_to_file(&vec![4], &path).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "[\n  4\n]\n");
        let names: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(names, ["state.json"]);
    }
}
