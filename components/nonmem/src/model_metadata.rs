use anyhow::{Result, anyhow, bail};
use fs_err as fs;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use utils::write_json_to_file;

pub const METADATA_FILENAME_SUFFIX: &str = "_metadata.json";

#[derive(Debug, Clone, Serialize, Deserialize, Default, Hash, PartialEq, Eq)]
pub struct ModelMetadata {
    /// Parent model(s) this model is based on
    #[serde(default)]
    pub based_on: Vec<String>,
    /// Short description of the model
    pub description: String,
    pub tags: Vec<String>,
}

impl ModelMetadata {
    pub fn new(based_on: Vec<String>, description: String) -> Result<Self> {
        if description.trim().is_empty() {
            bail!("Please provide a description for the model")
        }

        Ok(Self {
            based_on,
            description,
            tags: Vec::new(),
        })
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        Ok(serde_json::from_str(&content)?)
    }

    pub fn load_from_model_path(path: impl AsRef<Path>) -> Result<Self> {
        let model_path = resolve_model_path(&path)?;
        let (model_name, model_dir) = validate_model_path(&model_path)?;
        let metadata_path = model_dir.join(format!("{model_name}{METADATA_FILENAME_SUFFIX}"));

        Self::load(metadata_path)
    }

    pub fn save(&self, model_name: &str, folder: impl AsRef<Path>) -> Result<()> {
        if self.description.trim().is_empty() {
            bail!("No description was found in the metadata file")
        }

        let metadata_path = folder
            .as_ref()
            .join(format!("{model_name}{METADATA_FILENAME_SUFFIX}"));
        write_json_to_file(self, metadata_path)?;
        Ok(())
    }
    pub fn set(
        mut self,
        description: Option<String>,
        tags: Vec<String>,
        based_on: Vec<String>,
    ) -> Self {
        // Overwrite mode: replace fields that are provided
        if let Some(d) = description
            && !d.trim().is_empty()
        {
            self.description = d;
        }
        if !tags.is_empty() {
            self.tags = tags;
        }
        if !based_on.is_empty() {
            self.based_on = based_on;
        }
        self
    }
    pub fn update(
        mut self,
        description: Option<String>,
        tags: Vec<String>,
        based_on: Vec<String>,
    ) -> Self {
        // Append mode: merge with existing
        for tag in tags {
            if !self.tags.contains(&tag) {
                self.tags.push(tag)
            }
        }
        for based in based_on {
            if !self.based_on.contains(&based) {
                self.based_on.push(based)
            }
        }

        if let Some(d) = description {
            if self.description.trim().is_empty() {
                self.description = d
            } else if self.description.ends_with('.') {
                self.description = format!("{} {d}", self.description)
            } else {
                self.description = format!("{}. {d}", self.description);
            }
        }

        self
    }
}

// helper to check model path existence and get model name and model dir
pub fn validate_model_path(model_path: impl AsRef<Path>) -> Result<(String, PathBuf)> {
    let model_path = model_path.as_ref();
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
        .unwrap_or_else(|| std::path::Path::new("."))
        .to_owned();

    Ok((model_name, model_dir))
}

// helper to trim and remove empty elements
fn clean_vec(x: Vec<String>) -> Vec<String> {
    x.into_iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

// Validate that all based_on model files exist relative to the model directory
fn validate_based_on(based_on_vec: &Vec<String>, model_dir: impl AsRef<Path>) -> Result<()> {
    let model_dir = model_dir.as_ref();
    for based_on_path in based_on_vec {
        let full_path = model_dir.join(based_on_path);
        if !full_path.exists() {
            bail!(
                "Based-on model file does not exist: {based_on_path} (resolved to {})",
                full_path.display()
            );
        }
    }
    Ok(())
}

// helper to take metadata file and get mod/ctl file
fn resolve_model_path(input: impl AsRef<Path>) -> Result<PathBuf> {
    let input = input.as_ref();
    match input.extension().and_then(|e| e.to_str()) {
        Some("mod") | Some("ctl") => Ok(input.to_path_buf()),
        _ => {
            let name = input
                .file_name()
                .ok_or_else(|| anyhow!("no filename"))?
                .to_string_lossy();
            let base = name
                .strip_suffix(METADATA_FILENAME_SUFFIX)
                .ok_or_else(|| anyhow!("expected '*{METADATA_FILENAME_SUFFIX}'"))?;
            let dir = input.parent().unwrap_or_else(|| Path::new(""));

            let mod_path = dir.join(format!("{base}.mod"));
            if mod_path.exists() {
                return Ok(mod_path);
            }

            let ctl_path = dir.join(format!("{base}.ctl"));
            if ctl_path.exists() {
                return Ok(ctl_path);
            }

            bail!("no .mod or .ctl next to {}", input.to_string_lossy());
        }
    }
}

pub fn update_metadata_file(
    input: PathBuf,
    description: Option<String>,
    tags: Vec<String>,
    based_on: Vec<String>,
    overwrite: bool,
) -> Result<PathBuf> {
    let model_path = resolve_model_path(&input)?;
    let (model_name, model_dir) = validate_model_path(&model_path)?;
    let metadata_path = model_dir.join(format!("{model_name}{METADATA_FILENAME_SUFFIX}"));

    let tags_vec = clean_vec(tags);
    let based_on_vec = clean_vec(based_on);

    validate_based_on(&based_on_vec, &model_dir)?;

    let metadata = if metadata_path.exists() {
        let m = ModelMetadata::load(&metadata_path)?;
        if overwrite {
            m.set(description, tags_vec, based_on_vec)
        } else {
            m.update(description, tags_vec, based_on_vec)
        }
    } else {
        let mut m = ModelMetadata::new(based_on_vec, description.unwrap_or_default())?;
        m.tags = tags_vec;
        m
    };

    metadata.save(&model_name, &model_dir)?;
    Ok(metadata_path)
}

pub fn clear_metadata_file(
    model_name: String,
    model_dir: impl AsRef<Path>,
    metadata_path: impl AsRef<Path>,
    clear_based_on: bool,
    clear_tags: bool,
) -> Result<PathBuf> {
    let model_dir = model_dir.as_ref();
    let metadata_path = metadata_path.as_ref();

    let mut metadata = ModelMetadata::load(metadata_path)?;

    // Clear fields based on flags
    if clear_based_on {
        metadata.based_on.clear();
    }

    if clear_tags {
        metadata.tags.clear();
    }

    // Save updated metadata (description is preserved)
    metadata.save(&model_name, model_dir)?;
    Ok(metadata_path.to_path_buf())
}
