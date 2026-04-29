use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use fs_err as fs;
use nonmem_parser::Model;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Default, Serialize)]
pub struct Dataset {
    pub canonical_path: PathBuf,
    pub blake3_hash: String,
}

pub fn check_dataset(model: &Model, model_dir: &Path) -> Result<Dataset> {
    let p = model_dir.join(&model.data.path);
    if !p.exists() {
        bail!("Dataset {p:?} not found");
    }

    let data = fs::read(&p)?;
    let blake3_hash = format!("{}", blake3::hash(&data));

    Ok(Dataset {
        canonical_path: p.canonicalize()?,
        blake3_hash,
    })
}
