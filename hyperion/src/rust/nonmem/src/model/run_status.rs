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
