use std::fmt;
use std::path::Path;

use extendr_api::Result;

use crate::output_files::ext::create_ext_reader;
use hyperion_core::OptionExt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunStatus {
    Completed,
    RanWithErrors,
    NotRun,
}

impl fmt::Display for RunStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            RunStatus::Completed => "completed",
            RunStatus::RanWithErrors => "ran_with_errors",
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

    if !run_dir.exists() {
        return Ok(RunStatus::NotRun);
    }

    let ext_path = run_dir.join(format!("{}.ext", stem));
    if ext_path.exists() {
        let ext_reader = create_ext_reader(None, None, None, Some(true))?.final_estimates_only();
        if let Ok(tables) = ext_reader.parse_file(&ext_path) {
            if tables.iter().any(|table| !table.rows.is_empty()) {
                return Ok(RunStatus::Completed);
            }
        }
    }

    let lst_path = run_dir.join(format!("{}.lst", stem));
    if lst_path.exists() {
        return Ok(RunStatus::RanWithErrors);
    }

    Ok(RunStatus::NotRun)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn test_data_path() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("test_data")
            .join("run001")
            .join("run001.ext")
    }

    #[test]
    fn test_determine_run_status_completed() {
        let temp_dir = TempDir::new().unwrap();
        let mod_file = temp_dir.path().join("run001.mod");
        fs::write(&mod_file, "test content").unwrap();

        let run_dir = temp_dir.path().join("run001");
        fs::create_dir(&run_dir).unwrap();

        let ext_file = run_dir.join("run001.ext");
        fs::copy(test_data_path(), &ext_file).unwrap();

        let status = determine_run_status(&mod_file).unwrap();
        assert_eq!(status, RunStatus::Completed);
    }

    #[test]
    fn test_determine_run_status_ran_with_errors() {
        let temp_dir = TempDir::new().unwrap();
        let mod_file = temp_dir.path().join("run001.mod");
        fs::write(&mod_file, "test content").unwrap();

        let run_dir = temp_dir.path().join("run001");
        fs::create_dir(&run_dir).unwrap();

        let lst_file = run_dir.join("run001.lst");
        fs::write(&lst_file, "test content").unwrap();

        let status = determine_run_status(&mod_file).unwrap();
        assert_eq!(status, RunStatus::RanWithErrors);
    }

    #[test]
    fn test_determine_run_status_not_run() {
        let temp_dir = TempDir::new().unwrap();
        let mod_file = temp_dir.path().join("run001.mod");
        fs::write(&mod_file, "test content").unwrap();

        let status = determine_run_status(&mod_file).unwrap();
        assert_eq!(status, RunStatus::NotRun);
    }
}
