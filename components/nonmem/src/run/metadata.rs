use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow, bail};
use config::to_root_relative;
use fs_err as fs;
use serde::{Deserialize, Serialize};
use utils::write_json_to_file;

use crate::get_utc_now;
use crate::run::setup::ModelSetup;

pub const RUN_START_FILENAME: &str = "pharos_start.json";
pub const RUN_END_FILENAME: &str = "pharos_end.json";
pub const RUN_CONFIG_FILENAME: &str = "pharos_config.json";

/// Directory names skipped when walking a pharos project tree.
pub(crate) const SKIP_DIRS: &[&str] = &[".git", "rv"];

/// Recursively walk `root` and return the path of every `pharos_start.json`
/// found, skipping `SKIP_DIRS`.
fn walk_run_start_paths(root: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    let mut dirs = vec![root.to_path_buf()];
    while let Some(dir) = dirs.pop() {
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let file_type = entry.file_type()?;
            let name = entry.file_name().to_string_lossy().to_string();

            if file_type.is_dir() {
                if !SKIP_DIRS.contains(&name.as_str()) {
                    dirs.push(entry.path());
                }
                continue;
            }

            if file_type.is_file() && name == RUN_START_FILENAME {
                out.push(entry.path());
            }
        }
    }
    Ok(out)
}

/// Recursively walk `root` and return every `pharos_start.json` found,
/// paired with the directory that contains it.
pub(crate) fn walk_run_start_files(root: &Path) -> Result<Vec<(PathBuf, RunStartFile)>> {
    let mut out = Vec::new();
    for start_path in walk_run_start_paths(root)? {
        let dir = start_path
            .parent()
            .expect("start file path to have a parent dir");
        match RunStartFile::load(&start_path) {
            Ok(rs) => out.push((dir.to_path_buf(), rs)),
            Err(e) => eprintln!(
                "Warning: failed to load {}: {e}; skipping run metadata for this directory",
                start_path.display()
            ),
        }
    }
    Ok(out)
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct MigrationReport {
    pub migrated: usize,
    pub skipped: usize,
    pub failed: Vec<(PathBuf, String)>,
}

impl MigrationReport {
    /// Migrate pre-relative-path `pharos_start.json` files in place: replace
    /// the absolute `model_canonical_path` with a `model_path` relative to
    /// the project root. `base_path` is the project root the runs were
    /// originally recorded under (e.g. another user's home directory), used
    /// when the recorded paths are not under the current root.
    pub fn migrate_run_start_files(project_root: &Path, base_path: Option<&Path>) -> Result<Self> {
        let mut report = Self::default();
        for start_path in walk_run_start_paths(project_root)? {
            match Self::migrate_file(&start_path, project_root, base_path) {
                Ok(true) => report.migrated += 1,
                Ok(false) => report.skipped += 1,
                Err(e) => report.failed.push((start_path, e.to_string())),
            }
        }
        Ok(report)
    }

    /// Returns true if the file was migrated, false if it already parses as
    /// a current `RunStartFile` and was left untouched.
    fn migrate_file(path: &Path, project_root: &Path, base_path: Option<&Path>) -> Result<bool> {
        let content = fs::read_to_string(path)?;
        if serde_json::from_str::<RunStartFile>(&content).is_ok() {
            return Ok(false);
        }
        let legacy: LegacyRunStartFile = serde_json::from_str(&content)?;

        let canonical = &legacy.model_canonical_path;
        let rel = match to_root_relative(canonical, project_root) {
            Ok(rel) => rel,
            Err(_) => match base_path {
                Some(base) => to_root_relative(canonical, base).map_err(|_| {
                    anyhow!(
                        "'{}' is not under the project root or --base-path",
                        canonical.display()
                    )
                })?,
                None => bail!(
                    "'{}' is not under the project root; pass --base-path with the project root the run was recorded under",
                    canonical.display()
                ),
            },
        };

        if !project_root.join(&rel).exists() {
            bail!("model '{rel}' does not exist under the project root");
        }

        let dir = path.parent().expect("start file path to have a parent dir");
        legacy.into_run_start_file(rel).save(dir)?;
        Ok(true)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Hashes {
    pub blake3: String,
}

impl Hashes {
    pub fn formatted_blake3(&self) -> String {
        if self.blake3.len() > 8 {
            format!("{}...", &self.blake3[..8])
        } else {
            self.blake3.clone()
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OutputFileHash {
    pub filename: String,
    pub hashes: OutputHashes,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OutputHashes {
    pub blake3: String,
}

impl OutputHashes {
    pub fn formatted_blake3(&self) -> String {
        if self.blake3.len() > 8 {
            format!("{}...", &self.blake3[..8])
        } else {
            self.blake3.clone()
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RunStartFile {
    pub start: String,
    pub model_name: String,
    /// Model path relative to the `pharos.toml` project root, forward-slash separated.
    pub model_path: String,
    pub dataset_path: String,
    pub dataset_canonical_path: PathBuf,
    pub dataset_hashes: Hashes,
    pub model_hashes: Hashes,
}

impl RunStartFile {
    pub fn new(model_setup: &ModelSetup, model_path: String) -> Self {
        let start = get_utc_now();

        Self {
            start,
            model_path,
            model_name: model_setup.name.clone(),
            dataset_path: model_setup.dataset_original_path.clone(),
            dataset_canonical_path: model_setup.dataset_canonical_path.clone(),
            dataset_hashes: Hashes {
                blake3: model_setup.dataset_blake3_hash.clone(),
            },
            model_hashes: Hashes {
                blake3: model_setup.model_blake3_hash.clone(),
            },
        }
    }

    pub fn load(p: impl AsRef<Path>) -> Result<Self> {
        serde_json::from_reader(fs::File::open(p.as_ref())?).map_err(From::from)
    }

    pub fn save(&self, dir: impl AsRef<Path>) -> Result<()> {
        write_json_to_file(self, dir.as_ref().join(RUN_START_FILENAME))?;
        Ok(())
    }
}

/// Shape of `pharos_start.json` written before model paths went
/// project-relative; only exists so old files can be migrated.
#[derive(Deserialize)]
struct LegacyRunStartFile {
    start: String,
    model_name: String,
    model_canonical_path: PathBuf,
    dataset_path: String,
    dataset_canonical_path: PathBuf,
    dataset_hashes: Hashes,
    model_hashes: Hashes,
}

impl LegacyRunStartFile {
    /// `model_path` is the project-relative form of `model_canonical_path`,
    /// resolved by the caller.
    fn into_run_start_file(self, model_path: String) -> RunStartFile {
        RunStartFile {
            start: self.start,
            model_name: self.model_name,
            model_path,
            dataset_path: self.dataset_path,
            dataset_canonical_path: self.dataset_canonical_path,
            dataset_hashes: self.dataset_hashes,
            model_hashes: self.model_hashes,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RunEndFile {
    pub start: String,
    pub end: String,
    pub exit_code: i32,
    pub runtime_ms: u128,
    pub files_copied: HashSet<String>,
    pub output_files_rewrites: HashMap<String, String>,
    pub output_files_hashes: Vec<OutputFileHash>,
}

impl RunEndFile {
    pub fn load(p: impl AsRef<Path>) -> Result<Self> {
        serde_json::from_reader(fs::File::open(p.as_ref())?).map_err(From::from)
    }

    pub fn save(&self, dir: impl AsRef<Path>) -> Result<()> {
        write_json_to_file(self, dir.as_ref().join(RUN_END_FILENAME))?;
        Ok(())
    }

    pub fn formatted_runtime(&self) -> String {
        let seconds = self.runtime_ms as f64 / 1000.0;
        format!("{:.1}s", seconds)
    }
}
