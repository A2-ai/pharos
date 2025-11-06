use std::path::PathBuf;

use anyhow::{Result, anyhow, bail};

use crate::lineage::{METADATA_FILENAME_SUFFIX, ModelMetadata};

pub fn create_metadata_file(
    model_path: PathBuf,
    description: Option<String>,
    tags: Vec<String>,
    based_on: Vec<String>,
    overwrite: bool,
) -> Result<PathBuf> {
    if !model_path.exists() {
        bail!("Model file does not exist: {}", model_path.display());
    }

    let model_name = model_path
        .file_stem()
        .ok_or_else(|| anyhow!("Model file does not have a valid filename"))?
        .to_string_lossy()
        .to_string();

    let model_dir = model_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));

    let clean_vec = |x: Vec<String>| -> Vec<String> {
        x.into_iter()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    };
    let tags_vec = clean_vec(tags);
    let based_on_vec = clean_vec(based_on);

    // Validate that all based_on model files exist relative to the model directory
    for based_on_path in &based_on_vec {
        let full_path = model_dir.join(based_on_path);
        if !full_path.exists() {
            bail!(
                "Based-on model file does not exist: {} (resolved to {})",
                based_on_path,
                full_path.display()
            );
        }
    }

    let metadata_filename = format!("{model_name}{METADATA_FILENAME_SUFFIX}");
    let metadata_path = model_dir.join(&metadata_filename);
    if metadata_path.exists() && !overwrite {
        bail!("Metadata file '{metadata_filename}' already exists. Use --overwrite to replace it.");
    }

    // Create metadata instance
    let mut metadata = ModelMetadata::new(based_on_vec);
    metadata.description = description.unwrap_or_default();
    metadata.tags = tags_vec;

    // Save the metadata file in the same directory as the model
    metadata.save(&model_name, model_dir)?;

    Ok(metadata_path)
}
