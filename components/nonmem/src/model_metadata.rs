use anyhow::{Result, anyhow, bail};
use fs_err as fs;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::path::{Path, PathBuf};

use utils::write_json_to_file;

use crate::model_resolution::{resolve_model_path, resolve_model_reference};

pub const METADATA_FILENAME_SUFFIX: &str = "_metadata.json";

/// Normalize path-like strings at the serde boundary to forward slashes.
/// `PathBuf::to_string_lossy` produces backslashes on Windows, but pharos
/// uses forward-slash identifiers everywhere; normalizing on both read and
/// write keeps the on-disk format platform-independent regardless of how
/// the in-memory string was constructed.
fn deserialize_forward_slash<'de, D: Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    let s = String::deserialize(d)?;
    Ok(s.replace('\\', "/"))
}

fn deserialize_forward_slash_vec<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<String>, D::Error> {
    let v = Vec::<String>::deserialize(d)?;
    Ok(v.into_iter().map(|s| s.replace('\\', "/")).collect())
}

fn serialize_forward_slash<S: Serializer>(s: &str, ser: S) -> Result<S::Ok, S::Error> {
    ser.serialize_str(&s.replace('\\', "/"))
}

fn serialize_forward_slash_vec<S: Serializer>(v: &[String], ser: S) -> Result<S::Ok, S::Error> {
    use serde::ser::SerializeSeq;
    let mut seq = ser.serialize_seq(Some(v.len()))?;
    for s in v {
        seq.serialize_element(&s.replace('\\', "/"))?;
    }
    seq.end()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, Hash, PartialEq, Eq)]
pub struct ModelMetadata {
    /// Parent model(s) this model is based on
    #[serde(
        default,
        deserialize_with = "deserialize_forward_slash_vec",
        serialize_with = "serialize_forward_slash_vec"
    )]
    pub based_on: Vec<String>,
    /// Model this was mechanically copied from
    #[serde(
        default,
        deserialize_with = "deserialize_forward_slash",
        serialize_with = "serialize_forward_slash"
    )]
    pub copied_from: String,
    /// Short description of the model
    pub description: String,
    pub tags: Vec<String>,
}

impl ModelMetadata {
    pub fn new(
        based_on: Vec<String>,
        copied_from: String,
        description: String,
        tags: Vec<String>,
        model_dir: impl AsRef<Path>,
    ) -> Result<Self> {
        let model_dir = model_dir.as_ref();
        let description = description.trim().to_string();
        if description.is_empty() {
            bail!("Please provide a description for the model")
        }

        let based_on = resolve_vec(based_on, model_dir)?;
        let copied_from = resolve_opt(Some(copied_from), model_dir)?.unwrap_or_default();
        let tags = clean_vec(tags);

        Ok(Self {
            based_on,
            copied_from,
            description,
            tags,
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
        copied_from: Option<String>,
        model_dir: impl AsRef<Path>,
    ) -> Result<Self> {
        let model_dir = model_dir.as_ref();
        // Overwrite mode: replace fields that are provided
        if let Some(d) = clean_opt(description) {
            self.description = d;
        }
        let tags = clean_vec(tags);
        if !tags.is_empty() {
            self.tags = tags;
        }
        let based_on = resolve_vec(based_on, model_dir)?;
        if !based_on.is_empty() {
            self.based_on = based_on;
        }
        if let Some(c) = resolve_opt(copied_from, model_dir)? {
            self.copied_from = c;
        }
        Ok(self)
    }

    pub fn update(
        mut self,
        description: Option<String>,
        tags: Vec<String>,
        based_on: Vec<String>,
        model_dir: impl AsRef<Path>,
    ) -> Result<Self> {
        let model_dir = model_dir.as_ref();
        // Append mode: merge with existing
        for tag in clean_vec(tags) {
            if !self.tags.contains(&tag) {
                self.tags.push(tag)
            }
        }
        for resolved in resolve_vec(based_on, model_dir)? {
            if !self.based_on.contains(&resolved) {
                self.based_on.push(resolved)
            }
        }

        if let Some(d) = clean_opt(description) {
            if self.description.trim().is_empty() {
                self.description = d
            } else if self.description.trim().ends_with('.') {
                self.description = format!("{} {d}", self.description)
            } else {
                self.description = format!("{}. {d}", self.description);
            }
        }

        Ok(self)
    }
}

// helper to trim each entry and drop empties
fn clean_vec(v: Vec<String>) -> Vec<String> {
    v.into_iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

// helper to trim and drop if empty
fn clean_opt(o: Option<String>) -> Option<String> {
    o.map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

// helper to clean and resolve each entry against model_dir
fn resolve_vec(v: Vec<String>, model_dir: impl AsRef<Path>) -> Result<Vec<String>> {
    let model_dir = model_dir.as_ref();
    clean_vec(v)
        .into_iter()
        .map(|s| resolve_model_reference(&s, model_dir))
        .collect()
}

// helper to clean and resolve against model_dir; None if input is empty/whitespace
fn resolve_opt(o: Option<String>, model_dir: impl AsRef<Path>) -> Result<Option<String>> {
    let model_dir = model_dir.as_ref();
    clean_opt(o)
        .map(|s| resolve_model_reference(&s, model_dir))
        .transpose()
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

    let model_dir = model_path.parent().ok_or_else(|| {
        anyhow!(
            "Model path '{}' has no parent directory",
            model_path.display()
        )
    })?;
    let model_dir = if model_dir.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        model_dir.to_owned()
    };

    Ok((model_name, model_dir))
}

pub fn update_metadata_file(
    input: PathBuf,
    description: Option<String>,
    tags: Vec<String>,
    based_on: Vec<String>,
    copied_from: Option<String>,
    overwrite: bool,
) -> Result<PathBuf> {
    let model_path = resolve_model_path(&input)?;
    let (model_name, model_dir) = validate_model_path(&model_path)?;
    let metadata_path = model_dir.join(format!("{model_name}{METADATA_FILENAME_SUFFIX}"));

    let metadata = if metadata_path.exists() {
        let m = ModelMetadata::load(&metadata_path)?;
        if overwrite {
            m.set(description, tags, based_on, copied_from, &model_dir)?
        } else {
            if copied_from.is_some() {
                bail!("copied_from cannot be appended; rerun with --overwrite to replace it");
            }
            m.update(description, tags, based_on, &model_dir)?
        }
    } else {
        ModelMetadata::new(
            based_on,
            copied_from.unwrap_or_default(),
            description.unwrap_or_default(),
            tags,
            &model_dir,
        )?
    };

    metadata.save(&model_name, &model_dir)?;
    Ok(metadata_path)
}

pub fn clear_metadata_file(
    model_name: String,
    model_dir: impl AsRef<Path>,
    metadata_path: impl AsRef<Path>,
    clear_based_on: bool,
    clear_copied_from: bool,
    clear_tags: bool,
) -> Result<PathBuf> {
    let model_dir = model_dir.as_ref();
    let metadata_path = metadata_path.as_ref();

    let mut metadata = ModelMetadata::load(metadata_path)?;

    if clear_based_on {
        metadata.based_on.clear();
    }

    if clear_copied_from {
        metadata.copied_from.clear();
    }

    if clear_tags {
        metadata.tags.clear();
    }

    metadata.save(&model_name, model_dir)?;
    Ok(metadata_path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_rejects_empty_description() {
        assert!(
            ModelMetadata::new(vec![], String::new(), String::new(), vec![], Path::new(""))
                .is_err()
        );
        assert!(
            ModelMetadata::new(vec![], String::new(), "   ".into(), vec![], Path::new("")).is_err()
        );
    }

    #[test]
    fn test_path_strings_normalize_to_forward_slash() {
        // Simulate metadata written on Windows where PathBuf::to_string_lossy
        // produced backslashes. Forward-slash normalization happens at the
        // serde boundary, so the in-memory values must have forward slashes
        // on load, and serializing them must also write forward slashes
        // regardless of how the in-memory string was constructed.
        let json = r#"{
            "based_on": ["model\\nonmem\\base\\100.mod", "model\\nonmem\\base\\102.mod"],
            "copied_from": "model\\nonmem\\struct\\1001.mod",
            "description": "x",
            "tags": []
        }"#;

        let meta: ModelMetadata = serde_json::from_str(json).unwrap();
        assert_eq!(
            meta.based_on,
            vec!["model/nonmem/base/100.mod", "model/nonmem/base/102.mod"]
        );
        assert_eq!(meta.copied_from, "model/nonmem/struct/1001.mod");

        // Also confirm in-memory backslashes get rewritten on save.
        let mut sneaky = ModelMetadata::default();
        sneaky.based_on = vec!["a\\b\\c.mod".to_string()];
        sneaky.copied_from = "x\\y.mod".to_string();
        let serialized = serde_json::to_string(&sneaky).unwrap();
        assert!(serialized.contains(r#""based_on":["a/b/c.mod"]"#));
        assert!(serialized.contains(r#""copied_from":"x/y.mod""#));
    }
}
