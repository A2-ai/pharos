use std::fmt;
use std::path::Path;

use extendr_api::Result;

use hyperion_core::OptionExt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunStatus {
    Run,
    NotRun,
}

impl fmt::Display for RunStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            RunStatus::Run => "run",
            RunStatus::NotRun => "not_run",
        };
        f.write_str(value)
    }
}

pub fn determine_run_status(path: impl AsRef<Path>) -> Result<RunStatus> {
    let path = path.as_ref();
    let stem = path
        .file_stem()
        .ok_or_extendr_err("Could not determine model file stem")?
        .to_string_lossy()
        .to_string();
    let parent = path
        .parent()
        .ok_or_extendr_err("Could not determine model file parent directory")?;
    let run_dir = match path.extension().and_then(|e| e.to_str()) {
        Some("lst") => parent.to_path_buf(),
        _ => parent.join(&stem),
    };

    if run_dir.exists() {
        Ok(RunStatus::Run)
    } else {
        Ok(RunStatus::NotRun)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_determine_run_status_run() {
        let temp_dir = TempDir::new().unwrap();
        let mod_file = temp_dir.path().join("run001.mod");
        fs::write(&mod_file, "test content").unwrap();

        let run_dir = temp_dir.path().join("run001");
        fs::create_dir(&run_dir).unwrap();

        let status = determine_run_status(&mod_file).unwrap();
        assert_eq!(status, RunStatus::Run);
    }

    #[test]
    fn test_determine_run_status_not_run() {
        let temp_dir = TempDir::new().unwrap();
        let mod_file = temp_dir.path().join("run001.mod");
        fs::write(&mod_file, "test content").unwrap();

        let status = determine_run_status(&mod_file).unwrap();
        assert_eq!(status, RunStatus::NotRun);
    }

    #[test]
    fn test_determine_run_status_lst_file() {
        let temp_dir = TempDir::new().unwrap();
        let lst_file = temp_dir.path().join("run001.lst");
        fs::write(&lst_file, "test content").unwrap();

        // For .lst files, the run_dir is the parent directory itself
        // which exists, so status should be Run
        let status = determine_run_status(&lst_file).unwrap();
        assert_eq!(status, RunStatus::Run);
    }
}
