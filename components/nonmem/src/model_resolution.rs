use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use config::{render_output_dir_template, to_config_relative, to_root_relative};
use fs_err as fs;

use crate::model_metadata::METADATA_FILENAME_SUFFIX;
use crate::run::metadata::{RUN_START_FILENAME, RunStartFile, walk_run_start_files};

/// Validate that a model file path ends in `.mod` or `.ctl` and returns its extension.
pub fn validate_model_extension(path: &Path) -> Result<&'static str> {
    match path.extension().and_then(|e| e.to_str()) {
        Some("mod") => Ok("mod"),
        Some("ctl") => Ok("ctl"),
        Some(other) => bail!(
            "Model file {} has unsupported extension '.{}': only .mod and .ctl are allowed",
            path.display(),
            other
        ),
        None => bail!(
            "Model file {} has no extension: only .mod and .ctl are allowed",
            path.display()
        ),
    }
}

/// Helper to take metadata file and get mod/ctl file
pub(crate) fn resolve_model_path(input: impl AsRef<Path>) -> Result<PathBuf> {
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

            let reference = ModelReference::parse(base)?;
            reference
                .probe_candidates(&reference.candidates_at(dir))?
                .ok_or_else(|| anyhow!("no .mod or .ctl next to {}", input.to_string_lossy()))
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

/// The one true way to refer to a model, handles all validation/disambiguation
#[derive(Debug, Clone)]
pub struct ModelLayout {
    stem: String,
    extension: String,
    model_path: PathBuf,
    model_dir: PathBuf,
}

impl ModelLayout {
    /// Builds from a given existing model file.
    pub fn from_model_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let model_path = fs::canonicalize(path)
            .map_err(|e| anyhow!("Model file {} does not exist: {e}", path.display()))?;
        if !model_path.is_file() {
            bail!("{} is not a file", model_path.display());
        }

        let extension = validate_model_extension(&model_path)?.to_string();
        let stem = model_path
            .file_stem()
            .ok_or_else(|| anyhow!("Model file {} has no stem", model_path.display()))?
            .to_string_lossy()
            .to_string();
        let model_dir = model_path
            .parent()
            .ok_or_else(|| {
                anyhow!(
                    "Model file {} has no parent directory",
                    model_path.display()
                )
            })?
            .to_path_buf();

        Ok(Self {
            stem,
            extension,
            model_path,
            model_dir,
        })
    }

    /// Tries to find a model with bare name `reference` in the given `dir`
    /// Only errors if some paths do not exist. If everything is ok but no models were found, it
    /// will return `Ok(None)`
    pub fn try_locate(reference: &str, dir: impl AsRef<Path>) -> Result<Option<Self>> {
        let reference = ModelReference::parse(reference)?;
        match reference.probe_candidates(&reference.candidates_at(dir))? {
            Some(hit) => Ok(Some(Self::from_model_file(hit)?)),
            None => Ok(None),
        }
    }

    /// Tries to find the model given an output directory and the `pharos_start.json` file
    pub fn from_output_dir(dir: impl AsRef<Path>) -> Result<Self> {
        let dir_ref = dir.as_ref();
        let dir = fs::canonicalize(dir_ref)
            .map_err(|_| anyhow!("Directory does not exist: {}", dir_ref.display()))?;
        if !dir.is_dir() {
            bail!("Not a directory: {}", dir.display());
        }

        let start_path = dir.join(RUN_START_FILENAME);
        let stem = RunStartFile::load(&start_path)
            .with_context(|| {
                format!(
                    "Failed to read {} in {} (is this a pharos run output directory?)",
                    RUN_START_FILENAME,
                    dir.display()
                )
            })?
            .model_name;

        Self::try_locate(&stem, &dir)?.ok_or_else(|| {
            anyhow!(
                "Failed to find model file (.mod or .ctl) named '{stem}' in {}",
                dir.display()
            )
        })
    }

    pub fn stem(&self) -> &str {
        &self.stem
    }

    pub fn extension(&self) -> &str {
        &self.extension
    }

    pub fn model_path(&self) -> &Path {
        &self.model_path
    }

    pub fn model_dir(&self) -> &Path {
        &self.model_dir
    }

    /// Gives the path in `dir` with `{stem}.{ext}`
    pub fn output_file(&self, dir: impl AsRef<Path>, ext: &str) -> PathBuf {
        dir.as_ref().join(format!("{}.{ext}", self.stem))
    }

    /// Resolves output dir, optionally rendering a templated dir name
    pub fn resolve_output_dir(&self, template: Option<&str>) -> Result<PathBuf> {
        let name = match template {
            Some(t) => render_output_dir_template(t, &self.stem)?,
            None => self.stem.clone(),
        };
        Ok(self.model_dir.join(name))
    }

    /// Scan for output dir based on the `model_path`. It can find multiple matches
    /// if the output directory is templated with the timestamp.
    /// If we have more than one match, just error.
    ///
    /// `project_root` anchors the comparison: run-start files record the model
    /// path relative to the project root, so we relativize this model the same
    /// way and match relative-to-relative.
    pub fn discover_output_dir(&self, project_root: &Path) -> Result<Option<PathBuf>> {
        let key = to_root_relative(&self.model_path, project_root)?;
        let mut matches: Vec<PathBuf> = walk_run_start_files(&self.model_dir)?
            .into_iter()
            .filter(|(_, start)| start.model_path == key)
            .map(|(dir, _)| dir)
            .collect();

        match matches.len() {
            0 => Ok(None),
            1 => Ok(matches.pop()),
            _ => {
                matches.sort();
                let list = matches
                    .iter()
                    .map(|p| format!("  {}", p.display()))
                    .collect::<Vec<_>>()
                    .join("\n");
                bail!(
                    "Found multiple run outputs for {}:\n{list}\n",
                    self.model_path.display()
                )
            }
        }
    }
}

pub(crate) fn resolve_model_reference(input: &str, model_dir: impl AsRef<Path>) -> Result<String> {
    let model_dir = model_dir.as_ref();
    let cwd = std::env::current_dir()?;
    let reference = ModelReference::parse(input)?;
    let target = fs::canonicalize(reference.find(model_dir, &cwd)?)?;
    to_config_relative(&target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::run::metadata::Hashes;
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

    fn write_start_file(dir: &Path, model_name: &str, model_rel: &str) {
        RunStartFile {
            start: "2026-01-01T00:00:00+00:00".to_string(),
            model_name: model_name.to_string(),
            model_path: model_rel.to_string(),
            dataset_path: "data.csv".to_string(),
            dataset_canonical_path: dir.join("data.csv"),
            dataset_hashes: Hashes { blake3: "d".into() },
            model_hashes: Hashes { blake3: "m".into() },
        }
        .save(dir)
        .unwrap();
    }

    #[test]
    fn model_layout_try_locate() {
        let tmp = TempDir::new().unwrap();
        let dir = fs::canonicalize(tmp.path()).unwrap();
        touch(&dir, "run1.mod");

        let layout = ModelLayout::try_locate("run1", &dir).unwrap().unwrap();
        assert_eq!(layout.output_file(&dir, "ext"), dir.join("run1.ext"));
        assert!(ModelLayout::try_locate("missing", &dir).unwrap().is_none());
    }

    #[test]
    fn model_layout_from_output_dir() {
        let tmp = TempDir::new().unwrap();
        let run_dir = fs::canonicalize(tmp.path()).unwrap().join("run1_2026_fit");
        fs::create_dir_all(&run_dir).unwrap();
        touch(&run_dir, "run1.mod");
        let model = fs::canonicalize(run_dir.join("run1.mod")).unwrap();
        write_start_file(&run_dir, "run1", "run1.mod");
        let layout = ModelLayout::from_output_dir(&run_dir).unwrap();
        assert_eq!(layout.stem(), "run1");
        assert_eq!(layout.model_path(), model.as_path());
        assert_eq!(
            ModelLayout::from_output_dir(run_dir.join("."))
                .unwrap()
                .stem(),
            "run1"
        );

        // Missing or unreadable pharos_start.json is an error (no dir-name fallback).
        let tmp2 = TempDir::new().unwrap();
        let no_start = fs::canonicalize(tmp2.path()).unwrap().join("run2");
        fs::create_dir_all(&no_start).unwrap();
        touch(&no_start, "run2.mod");
        assert!(ModelLayout::from_output_dir(&no_start).is_err()); // missing
        fs::write(no_start.join(RUN_START_FILENAME), "{ not json").unwrap();
        assert!(ModelLayout::from_output_dir(&no_start).is_err()); // corrupt
    }

    #[test]
    fn model_layout_discover_output_dir() {
        let tmp = TempDir::new().unwrap();
        let root = fs::canonicalize(tmp.path()).unwrap();
        touch(&root, "run1.mod");
        let model = fs::canonicalize(root.join("run1.mod")).unwrap();
        let layout = ModelLayout::from_model_file(&model).unwrap();

        // No runs yet.
        assert!(layout.discover_output_dir(&root).unwrap().is_none());

        // Exactly one recorded run (dir name != stem, timestamped) is found by
        // the recorded model path, without re-rendering any template.
        let ts_dir = root.join("run1_2026-07-09T12_00_00");
        fs::create_dir_all(&ts_dir).unwrap();
        write_start_file(&ts_dir, "run1", "run1.mod");
        assert_eq!(layout.discover_output_dir(&root).unwrap().unwrap(), ts_dir);

        // A second run of the same model is ambiguous: error, don't guess.
        let ts_dir2 = root.join("run1_2026-07-10T12_00_00");
        fs::create_dir_all(&ts_dir2).unwrap();
        write_start_file(&ts_dir2, "run1", "run1.mod");
        assert!(layout.discover_output_dir(&root).is_err());

        // Runs for a different model don't count toward this model's matches:
        // `other` still resolves to its single run.
        touch(&root, "other.mod");
        let other = fs::canonicalize(root.join("other.mod")).unwrap();
        let other_dir = root.join("other_run");
        fs::create_dir_all(&other_dir).unwrap();
        write_start_file(&other_dir, "other", "other.mod");
        let other_layout = ModelLayout::from_model_file(&other).unwrap();
        assert_eq!(
            other_layout.discover_output_dir(&root).unwrap().unwrap(),
            other_dir
        );
    }

    #[test]
    fn resolver_scenarios() {
        enum Expect {
            InModelDir(&'static str),
            InCwd(&'static str),
            Fails(&'static [&'static str]),
        }
        use Expect::*;

        // (name, files in model_dir, files in cwd, input, expected outcome)
        #[rustfmt::skip]
        let cases: &[(&str, &[&str], &[&str], &str, Expect)] = &[
            // resolves in model_dir
            ("bare_to_mod", &["1010a.mod"], &[], "1010a", InModelDir("1010a.mod")),
            ("bare_to_ctl", &["1010a.ctl"], &[], "1010a", InModelDir("1010a.ctl")),
            ("bare_in_subdir", &["parents/p1.mod"], &[], "parents/p1", InModelDir("parents/p1.mod")),
            ("explicit_mod", &["parent.mod"], &[], "parent.mod", InModelDir("parent.mod")),
            ("ext_disambiguates", &["parent.mod", "parent.ctl"], &[], "parent.ctl", InModelDir("parent.ctl")),
            // cwd fallback when model_dir is empty
            ("cwd_explicit_subdir", &[], &["tmp/struct/1001.mod"], "tmp/struct/1001.mod", InCwd("tmp/struct/1001.mod")),
            ("cwd_bare_to_mod", &[], &["1010a.mod"], "1010a", InCwd("1010a.mod")),
            ("cwd_bare_to_ctl", &[], &["1010a.ctl"], "1010a", InCwd("1010a.ctl")),
            ("cwd_bare_in_subdir", &[], &["tmp/struct/1002.mod"], "tmp/struct/1002", InCwd("tmp/struct/1002.mod")),
            // failures
            ("ambig_in_model_dir", &["1010a.mod", "1010a.ctl"], &[], "1010a", Fails(&["1010a.mod", "1010a.ctl", "ambig"])),
            ("ambig_in_cwd", &[], &["1010a.mod", "1010a.ctl"], "1010a", Fails(&["1010a.mod", "1010a.ctl", "ambig"])),
            ("bare_missing_everywhere", &[], &[], "1010a", Fails(&["1010a"])),
            ("explicit_missing", &[], &[], "parent.mod", Fails(&["parent.mod"])),
            ("rejects_bad_ext", &["parent.txt"], &[], "parent.txt", Fails(&["unsupported extension", ".txt"])),
            ("rejects_bad_ext_subdir", &["parents/p1.yaml"], &[], "parents/p1.yaml", Fails(&["unsupported extension", ".yaml"])),
        ];

        for (name, in_model_dir, in_cwd, input, expect) in cases {
            let tmp = TempDir::new().unwrap();
            let model_dir = tmp.path().join("m");
            let cwd = tmp.path().join("c");
            fs::create_dir_all(&model_dir).unwrap();
            fs::create_dir_all(&cwd).unwrap();
            for f in *in_model_dir {
                touch_rel(&model_dir, f);
            }
            for f in *in_cwd {
                touch_rel(&cwd, f);
            }

            let result = ModelReference::parse(input).and_then(|r| r.find(&model_dir, &cwd));
            let label = format!("[{name}]");

            match expect {
                InModelDir(target) | InCwd(target) => {
                    let resolved =
                        result.unwrap_or_else(|e| panic!("{label} expected Ok, got Err: {e}"));
                    let anchor_dir = match expect {
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
