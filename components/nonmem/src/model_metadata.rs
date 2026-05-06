use anyhow::{Result, anyhow, bail};
use config::to_config_relative;
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

            let mod_name = format!("{base}.mod");
            let ctl_name = format!("{base}.ctl");
            let mod_path = dir.join(&mod_name);
            let ctl_path = dir.join(&ctl_name);
            match (mod_path.exists(), ctl_path.exists()) {
                (true, true) => bail!(
                    "Ambiguous model reference: both {mod_name} and {ctl_name} exist next to {}. \
                     Specify the extension explicitly.",
                    input.display()
                ),
                (true, false) => Ok(mod_path),
                (false, true) => Ok(ctl_path),
                (false, false) => bail!("no .mod or .ctl next to {}", input.to_string_lossy()),
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ModelReferenceKind {
    Absolute,
    ExplicitRelative,
    BareRelative,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ModelReference {
    kind: ModelReferenceKind,
    input: String,
    path: PathBuf,
}

impl ModelReference {
    fn parse(input: &str) -> Result<Self> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            bail!("Model reference cannot be empty");
        }

        let path = PathBuf::from(trimmed);
        let ext = path.extension().and_then(|e| e.to_str());

        let kind = match (ext, path.is_absolute()) {
            (Some("mod") | Some("ctl"), true) => ModelReferenceKind::Absolute,
            (Some("mod") | Some("ctl"), false) => ModelReferenceKind::ExplicitRelative,
            (None, false) => ModelReferenceKind::BareRelative,
            (other, _) => {
                let ext_display = other.map(|e| format!(".{e}")).unwrap_or_default();
                bail!(
                    "Model reference '{trimmed}' has unsupported extension '{ext_display}': only .mod and .ctl are allowed"
                )
            }
        };

        Ok(Self {
            kind,
            input: trimmed.to_string(),
            path,
        })
    }

    fn candidates(&self) -> Vec<PathBuf> {
        match self.kind {
            ModelReferenceKind::Absolute | ModelReferenceKind::ExplicitRelative => {
                vec![self.path.clone()]
            }
            ModelReferenceKind::BareRelative => ["mod", "ctl"]
                .iter()
                .map(|ext| {
                    let mut c = self.path.clone();
                    c.set_extension(ext);
                    c
                })
                .collect(),
        }
    }

    fn candidates_at(&self, root: impl AsRef<Path>) -> Vec<PathBuf> {
        let root = root.as_ref();

        self.candidates()
            .into_iter()
            .map(|c| root.join(c))
            .collect()
    }

    fn probe_candidates(&self, paths: &[PathBuf]) -> Result<Option<PathBuf>> {
        let existing: Vec<PathBuf> = paths.iter().filter(|p| p.exists()).cloned().collect();
        match existing.as_slice() {
            [] => Ok(None),
            [single] => Ok(Some(single.clone())),
            multiple => {
                let names: Vec<String> = multiple.iter().map(|p| p.display().to_string()).collect();
                bail!(
                    "Ambiguous model reference '{}': both {} exist. \
                     Specify the extension explicitly.",
                    self.input,
                    names.join(" and ")
                )
            }
        }
    }

    fn find(&self, model_dir: impl AsRef<Path>, cwd: impl AsRef<Path>) -> Result<PathBuf> {
        let model_dir = model_dir.as_ref();
        let cwd = cwd.as_ref();

        match self.kind {
            // Absolute references probe their own location. No fallback.
            ModelReferenceKind::Absolute => self
                .probe_candidates(&self.candidates())?
                .ok_or_else(|| anyhow!("Model file does not exist: {}", self.input)),

            // Relative references probe model_dir first, then fall back to cwd.
            ModelReferenceKind::ExplicitRelative | ModelReferenceKind::BareRelative => {
                if let Some(hit) = self.probe_candidates(&self.candidates_at(model_dir))? {
                    return Ok(hit);
                }

                self.probe_candidates(&self.candidates_at(cwd))?
                    .ok_or_else(|| {
                        anyhow!(
                            "Model file does not exist: {} (searched in {} and {})",
                            self.input,
                            model_dir.display(),
                            cwd.display()
                        )
                    })
            }
        }
    }
}

fn resolve_model_reference(input: &str, model_dir: impl AsRef<Path>) -> Result<String> {
    let model_dir = model_dir.as_ref();
    let cwd = std::env::current_dir()?;
    let reference = ModelReference::parse(input)?;
    let target = fs::canonicalize(reference.find(model_dir, &cwd)?)?;
    let rel = to_config_relative(&target)?;
    Ok(rel.to_string_lossy().to_string())
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
    use tempfile::TempDir;

    fn touch(dir: impl AsRef<Path>, name: &str) {
        fs::write(dir.as_ref().join(name), "").unwrap();
    }

    fn touch_rel(root: impl AsRef<Path>, rel: &str) {
        let full = root.as_ref().join(rel);
        if let Some(parent) = full.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(full, "").unwrap();
    }

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

    /// Drives `ModelReference::parse → find` directly.
    /// Each case places files under `model_dir`, `cwd`, or both; the `expect`
    /// variant declares where resolution should land or which substrings the
    /// (lowercased) error should contain.
    #[test]
    fn resolver_scenarios() {
        enum Expect {
            InModelDir(&'static str),
            InCwd(&'static str),
            Fails(&'static [&'static str]),
        }
        struct Case {
            name: &'static str,
            in_model_dir: &'static [&'static str],
            in_cwd: &'static [&'static str],
            input: &'static str,
            expect: Expect,
        }
        impl Case {
            fn new(
                name: &'static str,
                in_model_dir: &'static [&'static str],
                in_cwd: &'static [&'static str],
                input: &'static str,
                expect: Expect,
            ) -> Self {
                Self {
                    name,
                    in_model_dir,
                    in_cwd,
                    input,
                    expect,
                }
            }
        }

        use Expect::*;

        #[rustfmt::skip]
        let cases = &[
            // resolves in model_dir
            Case::new("bare_to_mod", &["1010a.mod"], &[], "1010a", InModelDir("1010a.mod")),
            Case::new("bare_to_ctl", &["1010a.ctl"], &[], "1010a", InModelDir("1010a.ctl")),
            Case::new("bare_in_subdir", &["parents/p1.mod"], &[], "parents/p1", InModelDir("parents/p1.mod")),
            Case::new("explicit_mod", &["parent.mod"], &[], "parent.mod", InModelDir("parent.mod")),
            Case::new("ext_disambiguates", &["parent.mod", "parent.ctl"], &[], "parent.ctl", InModelDir("parent.ctl")),
            // cwd fallback when model_dir is empty
            Case::new("cwd_explicit_subdir", &[], &["tmp/struct/1001.mod"], "tmp/struct/1001.mod", InCwd("tmp/struct/1001.mod")),
            Case::new("cwd_bare_to_mod", &[], &["1010a.mod"], "1010a", InCwd("1010a.mod")),
            Case::new("cwd_bare_to_ctl", &[], &["1010a.ctl"], "1010a", InCwd("1010a.ctl")),
            Case::new("cwd_bare_in_subdir", &[], &["tmp/struct/1002.mod"], "tmp/struct/1002", InCwd("tmp/struct/1002.mod")),
            // failures
            Case::new("ambig_in_model_dir", &["1010a.mod", "1010a.ctl"], &[], "1010a", Fails(&["1010a.mod", "1010a.ctl", "ambig"])),
            Case::new("ambig_in_cwd", &[], &["1010a.mod", "1010a.ctl"], "1010a", Fails(&["1010a.mod", "1010a.ctl", "ambig"])),
            Case::new("bare_missing_everywhere", &[], &[], "1010a", Fails(&["1010a"])),
            Case::new("explicit_missing", &[], &[], "parent.mod", Fails(&["parent.mod"])),
            Case::new("rejects_bad_ext", &["parent.txt"], &[], "parent.txt", Fails(&["unsupported extension", ".txt"])),
            Case::new("rejects_bad_ext_subdir", &["parents/p1.yaml"], &[], "parents/p1.yaml", Fails(&["unsupported extension", ".yaml"])),
        ];

        for case in cases {
            let tmp = TempDir::new().unwrap();
            let model_dir = tmp.path().join("m");
            let cwd = tmp.path().join("c");
            fs::create_dir_all(&model_dir).unwrap();
            fs::create_dir_all(&cwd).unwrap();
            for f in case.in_model_dir {
                touch_rel(&model_dir, f);
            }
            for f in case.in_cwd {
                touch_rel(&cwd, f);
            }

            let result = ModelReference::parse(case.input).and_then(|r| r.find(&model_dir, &cwd));
            let label = format!("[{}]", case.name);

            match &case.expect {
                InModelDir(target) | InCwd(target) => {
                    let resolved =
                        result.unwrap_or_else(|e| panic!("{label} expected Ok, got Err: {e}"));
                    let anchor_dir = match case.expect {
                        InModelDir(_) => &model_dir,
                        InCwd(_) => &cwd,
                        Fails(_) => unreachable!(),
                    };
                    assert_eq!(
                        resolved.canonicalize().unwrap(),
                        anchor_dir.join(target).canonicalize().unwrap(),
                        "{label}",
                    );
                }
                Fails(needles) => {
                    let err = match result {
                        Ok(p) => panic!("{label} expected Err, got Ok({})", p.display()),
                        Err(e) => format!("{e}").to_lowercase(),
                    };
                    for n in *needles {
                        assert!(
                            err.contains(&n.to_lowercase()),
                            "{label} '{err}' missing '{n}'"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn resolve_model_path_ambiguous_siblings() {
        let tmp = TempDir::new().unwrap();
        touch(tmp.path(), "1010_metadata.json");
        touch(tmp.path(), "1010.mod");
        touch(tmp.path(), "1010.ctl");

        let err = format!(
            "{}",
            resolve_model_path(tmp.path().join("1010_metadata.json")).unwrap_err()
        )
        .to_lowercase();
        assert!(err.contains("1010.mod") && err.contains("1010.ctl") && err.contains("ambig"));
    }
}
