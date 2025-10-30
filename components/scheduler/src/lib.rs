use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::Result;
use fs_err as fs;

const SUBMISSIONS_DIR: &str = "submission-log";

pub(crate) fn get_or_create_submissions_dir(parent: impl AsRef<Path>) -> Result<PathBuf> {
    let dir = parent.as_ref().join(SUBMISSIONS_DIR);
    fs::create_dir_all(&dir)?;
    Ok(dir.canonicalize()?)
}

pub(crate) fn get_or_create_logs_dir(
    parent: impl AsRef<Path>,
    passed_log_dir: Option<PathBuf>,
    default_logs_dir: &str,
) -> Result<PathBuf> {
    let dir = if let Some(d) = passed_log_dir {
        d
    } else {
        parent.as_ref().join(default_logs_dir)
    };

    if dir.exists() {
        return Ok(dir);
    }

    fs::create_dir_all(&dir)?;
    let gitignore = dir.join(".gitignore");
    let mut f = fs::File::create(gitignore)?;
    f.write_all(b"*\n!.gitignore")?;

    Ok(dir.canonicalize()?)
}

pub mod sge;
pub mod slurm;
