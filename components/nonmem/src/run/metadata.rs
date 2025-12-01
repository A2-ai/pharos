use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use anyhow::Result;
use fs_err as fs;
use serde::{Deserialize, Serialize};
use utils::write_json_to_file;

use crate::get_utc_now;
use crate::run::setup::ModelSetup;

pub const RUN_START_FILENAME: &str = "pharos_start.json";
pub const RUN_END_FILENAME: &str = "pharos_end.json";
pub const RUN_CONFIG_FILENAME: &str = "pharos_config.json";

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
    pub model_canonical_path: PathBuf,
    pub dataset_path: String,
    pub dataset_canonical_path: PathBuf,
    pub dataset_hashes: Hashes,
    pub model_hashes: Hashes,
}

impl RunStartFile {
    pub fn new(model_setup: &ModelSetup, model_canonical_path: &Path) -> Self {
        let start = get_utc_now();

        Self {
            start,
            model_canonical_path: model_canonical_path.to_path_buf(),
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
