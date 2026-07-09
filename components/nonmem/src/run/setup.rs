use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use config::{NonmemConfig, render_output_dir_template};
use fs_err as fs;
use nonmem_parser::Model;

use crate::dataset::check_dataset;

#[derive(Debug, Default)]
pub struct ModelSetup {
    pub name: String,
    pub extension: String,
    pub model_dir: PathBuf,
    pub output_dir: PathBuf,
    pub dataset_original_path: String,
    pub dataset_canonical_path: PathBuf,
    pub dataset_blake3_hash: String,
    pub model_blake3_hash: String,
    /// This will point to the canonicalized dataset path
    pub model_content: String,
    /// original -> new location
    pub output_files: HashMap<String, String>,
}

pub fn prepare_model(
    path: &Path,
    overwrite: bool,
    output_dir: Option<String>,
    config: &NonmemConfig,
) -> Result<ModelSetup> {
    let path = path.canonicalize()?;
    if !path.exists() {
        bail!("Model file {} does not exist", path.display());
    }
    if !path.is_file() {
        bail!("{} is not a file", path.display());
    }

    let parent_dir = path.parent().expect("models to not be at the root of FS");
    let model_name = path
        .file_stem()
        .expect("models to have a filename")
        .to_string_lossy()
        .to_string(); // e.g., "run001"

    let extension = crate::model_metadata::validate_model_extension(&path)?.to_string();

    let output_dir_name = if let Some(o) = output_dir {
        render_output_dir_template(&o, &model_name)?
    } else {
        model_name.clone()
    };

    let output_dir = parent_dir.join(output_dir_name);
    if output_dir.is_dir() {
        if overwrite {
            if !output_dir.starts_with(parent_dir) || output_dir == parent_dir {
                bail!(
                    "Cannot overwrite {output_dir:?}: outside the model directory or is the parent directory."
                );
            }
            fs::remove_dir_all(&output_dir)?;
        } else {
            bail!("Output directory already exists and --overwrite is not enabled.")
        }
    }

    // Read and hash the original model file
    let model_content = fs::read_to_string(&path)?;
    let model_blake3_hash = format!("{}", blake3::hash(model_content.as_bytes()));

    let mut setup = ModelSetup {
        name: model_name,
        extension,
        output_dir,
        model_blake3_hash,
        model_dir: parent_dir.to_path_buf(),
        ..Default::default()
    };

    let model = Model::parse(&path, &model_content)?;
    if let Some(comment_type) = config.comments.r#type {
        let failed = model.validate_comments(comment_type);
        if !failed.is_empty() && config.comments.error_on_invalid {
            bail!(
                "\nSome comments are not matching the expected type: \n{}",
                failed.join("\n")
            );
        }
    }
    setup.dataset_original_path = model.data.path.clone();
    let dataset = check_dataset(&model, parent_dir)?;
    setup.output_files = model.paths_to_replace();
    setup.model_content = model.with_modified_paths(&dataset.canonical_path);
    setup.dataset_canonical_path = dataset.canonical_path;
    setup.dataset_blake3_hash = dataset.blake3_hash;

    Ok(setup)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn write_model(dir: &Path, file_name: &str) -> PathBuf {
        let model_path = dir.join(file_name);
        let content = "$PROBLEM tiny\n$INPUT ID TIME DV\n$DATA data.csv\n$THETA 1\n$SIGMA 1\n";
        fs::write(&model_path, content).unwrap();
        fs::write(dir.join("data.csv"), "id,time,dv\n1,0,0\n").unwrap();
        model_path
    }

    #[test]
    fn prepare_model_preserves_mod_extension() {
        let tmp = tempdir().unwrap();
        let model = write_model(tmp.path(), "model.mod");
        let setup = prepare_model(&model, false, None, &NonmemConfig::default()).unwrap();
        assert_eq!(setup.extension, "mod");
        assert_eq!(setup.name, "model");

        let model = write_model(tmp.path(), "model.ctl");
        let setup = prepare_model(&model, false, None, &NonmemConfig::default()).unwrap();
        assert_eq!(setup.extension, "ctl");
        assert_eq!(setup.name, "model");

        let model = write_model(tmp.path(), "model.txt");
        let setup = prepare_model(&model, false, None, &NonmemConfig::default());
        assert!(setup.is_err());
    }
}
