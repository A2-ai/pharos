use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use config::{CommentsConfig, render_output_template};
use fs_err as fs;

use crate::parsing::Model;

#[derive(Debug, Default)]
pub struct ModelSetup {
    pub name: String,
    pub output_dir: PathBuf,
    pub dataset_original_path: String,
    pub dataset_canonical_path: PathBuf,
    pub dataset_blake3_hash: String,
    pub model_blake3_hash: String,
    /// This will point to the canonicalized dataset path
    pub model_content: String,
    // original -> new location
    pub output_files: HashMap<String, String>,
}

pub fn prepare_model(
    path: &Path,
    overwrite: bool,
    output_dir: Option<String>,
    comments_config: &CommentsConfig,
) -> Result<ModelSetup> {
    let path = path.canonicalize()?;
    if !path.exists() {
        bail!("Model file {} does not exist", path.display());
    }
    if !path.is_file() {
        bail!("{} is not a file", path.display());
    }

    let parent_dir = path.parent().expect("models to not be at the root of FS");
    let file_name = path.file_name().expect("models to have a filename");
    let model_name = file_name.to_string_lossy().to_string();

    let output_dir_name = if let Some(o) = output_dir {
        render_output_template(&o, &model_name)?
    } else {
        model_name.to_string()
    };

    let output_dir = parent_dir.join(output_dir_name);
    if output_dir.is_dir() {
        if overwrite {
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
        output_dir,
        model_blake3_hash,
        ..Default::default()
    };

    let mut model = Model::parse(&model_content)?;
    if let Some(comment_type) = comments_config.r#type {
        let failed = model.parse_comments(comment_type);
        if !failed.is_empty() && comments_config.error_on_invalid {
            bail!(
                "\nSome comments are not matching the expected type: \n{}",
                failed.join("\n")
            );
        }
    }
    setup.dataset_original_path = model.data.path.clone();
    let dataset = model.check_dataset(parent_dir)?;
    setup.output_files = model.paths_to_replace();
    setup.model_content = model.with_modified_paths(&dataset.canonical_path);
    setup.dataset_canonical_path = dataset.canonical_path;
    setup.dataset_blake3_hash = dataset.blake3_hash;

    Ok(setup)
}
