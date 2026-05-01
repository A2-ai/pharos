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
    /// Model this was mechanically copied from
    #[serde(default)]
    pub copied_from: String,
    /// Short description of the model
    pub description: String,
    pub tags: Vec<String>,
}

impl ModelMetadata {
    pub fn new(based_on: Vec<String>, copied_from: String, description: String) -> Result<Self> {
        if description.trim().is_empty() {
            bail!("Please provide a description for the model")
        }

        Ok(Self {
            based_on,
            copied_from,
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
        copied_from: Option<String>,
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
        if let Some(c) = copied_from
            && !c.trim().is_empty()
        {
            self.copied_from = c;
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

fn clean_opt(x: Option<String>) -> Option<String> {
    x.map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

fn validate_relative_path_exists(rel: &str, model_dir: &Path) -> Result<()> {
    let full_path = model_dir.join(rel);
    if !full_path.exists() {
        bail!(
            "Model file does not exist: {rel} (resolved to {})",
            full_path.display()
        );
    }
    Ok(())
}

pub(crate) fn validate_relative_paths_exist(
    paths: &[String],
    model_dir: impl AsRef<Path>,
) -> Result<()> {
    let model_dir = model_dir.as_ref();
    for p in paths {
        validate_relative_path_exists(p, model_dir)?;
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
    copied_from: Option<String>,
    overwrite: bool,
) -> Result<PathBuf> {
    let model_path = resolve_model_path(&input)?;
    let (model_name, model_dir) = validate_model_path(&model_path)?;
    let metadata_path = model_dir.join(format!("{model_name}{METADATA_FILENAME_SUFFIX}"));

    let tags_vec = clean_vec(tags);
    let based_on_vec = clean_vec(based_on);
    let copied_from = clean_opt(copied_from);

    validate_relative_paths_exist(&based_on_vec, &model_dir)?;
    if let Some(cf) = &copied_from {
        validate_relative_path_exists(cf, &model_dir)?;
    }

    let metadata = if metadata_path.exists() {
        let m = ModelMetadata::load(&metadata_path)?;
        if overwrite {
            m.set(description, tags_vec, based_on_vec, copied_from)
        } else {
            m.update(description, tags_vec, based_on_vec)
        }
    } else {
        let mut m = ModelMetadata::new(
            based_on_vec,
            copied_from.unwrap_or_default(),
            description.unwrap_or_default(),
        )?;
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
    use tempfile::TempDir;

    fn touch(dir: &Path, name: &str) {
        fs::write(dir.join(name), "").unwrap();
    }

    fn touch_rel(root: &Path, rel: &str) {
        let full = root.join(rel);
        if let Some(parent) = full.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(full, "").unwrap();
    }

    fn read_metadata(dir: &Path, model_name: &str) -> ModelMetadata {
        ModelMetadata::load(dir.join(format!("{model_name}{METADATA_FILENAME_SUFFIX}"))).unwrap()
    }

    #[test]
    fn new_rejects_empty_description() {
        assert!(ModelMetadata::new(vec![], String::new(), String::new()).is_err());
        assert!(ModelMetadata::new(vec![], String::new(), "   ".into()).is_err());
    }

    #[test]
    fn save_and_load_round_trip() {
        let tmp = TempDir::new().unwrap();
        let m =
            ModelMetadata::new(vec!["base.mod".into()], "src.mod".into(), "desc".into()).unwrap();
        m.save("1010", tmp.path()).unwrap();
        assert_eq!(read_metadata(tmp.path(), "1010"), m);
    }

    /// One scenario applies to both `based_on` and `copied_from` since they share
    /// the same resolver. Each case is run twice — once per field — to lock in symmetry.
    #[test]
    fn resolver_scenarios() {
        enum Expect {
            Resolved(&'static str),
            Fails(&'static [&'static str]),
        }

        struct ResolveCase {
            name: &'static str,
            sibling_files: &'static [&'static str],
            input: &'static str,
            expect: Expect,
        }

        let cases = &[
            ResolveCase {
                name: "bare_resolves_to_mod",
                sibling_files: &["1010a.mod"],
                input: "1010a",
                expect: Expect::Resolved("1010a.mod"),
            },
            ResolveCase {
                name: "bare_resolves_to_ctl",
                sibling_files: &["1010a.ctl"],
                input: "1010a",
                expect: Expect::Resolved("1010a.ctl"),
            },
            ResolveCase {
                name: "bare_ambiguous",
                sibling_files: &["1010a.mod", "1010a.ctl"],
                input: "1010a",
                expect: Expect::Fails(&["1010a.mod", "1010a.ctl", "ambig"]),
            },
            ResolveCase {
                name: "bare_missing",
                sibling_files: &[],
                input: "1010a",
                expect: Expect::Fails(&["1010a"]),
            },
            ResolveCase {
                name: "bare_in_subdir_resolves",
                sibling_files: &["parents/p1.mod"],
                input: "parents/p1",
                expect: Expect::Resolved("parents/p1.mod"),
            },
            ResolveCase {
                name: "full_mod_unchanged",
                sibling_files: &["parent.mod"],
                input: "parent.mod",
                expect: Expect::Resolved("parent.mod"),
            },
            ResolveCase {
                name: "full_extension_disambiguates",
                sibling_files: &["parent.mod", "parent.ctl"],
                input: "parent.ctl",
                expect: Expect::Resolved("parent.ctl"),
            },
            ResolveCase {
                name: "full_missing",
                sibling_files: &[],
                input: "parent.mod",
                expect: Expect::Fails(&["parent.mod"]),
            },
        ];

        for case in cases {
            for field in ["based_on", "copied_from"] {
                let tmp = TempDir::new().unwrap();
                touch(tmp.path(), "child.mod");
                for f in case.sibling_files {
                    touch_rel(tmp.path(), f);
                }

                let (based_on_arg, copied_from_arg) = if field == "based_on" {
                    (vec![case.input.to_string()], None)
                } else {
                    (vec![], Some(case.input.to_string()))
                };

                let result = update_metadata_file(
                    tmp.path().join("child.mod"),
                    Some("desc".into()),
                    vec![],
                    based_on_arg,
                    copied_from_arg,
                    true,
                );

                let label = format!("[{}/{field}]", case.name);
                match &case.expect {
                    Expect::Resolved(resolved) => {
                        result.unwrap_or_else(|e| panic!("{label} expected Ok, got Err: {e}"));
                        let m = read_metadata(tmp.path(), "child");
                        if field == "based_on" {
                            assert_eq!(m.based_on, vec![resolved.to_string()], "{label}");
                        } else {
                            assert_eq!(m.copied_from, *resolved, "{label}");
                        }
                    }
                    Expect::Fails(needles) => {
                        let err = match result {
                            Ok(_) => panic!("{label} expected Err, got Ok"),
                            Err(e) => format!("{e}").to_lowercase(),
                        };
                        for needle in *needles {
                            assert!(
                                err.contains(&needle.to_lowercase()),
                                "{label} error '{err}' missing '{needle}'"
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn based_on_multi_entry_each_resolved() {
        let tmp = TempDir::new().unwrap();
        touch(tmp.path(), "child.mod");
        touch(tmp.path(), "p1.mod");
        touch(tmp.path(), "p2.ctl");

        update_metadata_file(
            tmp.path().join("child.mod"),
            Some("desc".into()),
            vec![],
            vec!["p1".into(), "p2".into()],
            None,
            true,
        )
        .unwrap();

        assert_eq!(
            read_metadata(tmp.path(), "child").based_on,
            vec!["p1.mod".to_string(), "p2.ctl".to_string()]
        );
    }

    #[test]
    fn based_on_resolves_in_append_mode() {
        let tmp = TempDir::new().unwrap();
        touch(tmp.path(), "child.mod");
        touch(tmp.path(), "p1.mod");
        touch(tmp.path(), "p2.ctl");

        update_metadata_file(
            tmp.path().join("child.mod"),
            Some("desc".into()),
            vec![],
            vec!["p1".into()],
            None,
            true,
        )
        .unwrap();

        update_metadata_file(
            tmp.path().join("child.mod"),
            None,
            vec![],
            vec!["p2".into()],
            None,
            false,
        )
        .unwrap();

        assert_eq!(
            read_metadata(tmp.path(), "child").based_on,
            vec!["p1.mod".to_string(), "p2.ctl".to_string()]
        );
    }
}
