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
    pub fn new(
        based_on: Vec<String>,
        copied_from: String,
        description: String,
        model_dir: &Path,
    ) -> Result<Self> {
        if description.trim().is_empty() {
            bail!("Please provide a description for the model")
        }

        let based_on = based_on
            .into_iter()
            .map(|b| resolve_model_reference(&b, model_dir))
            .collect::<Result<_>>()?;

        let copied_from = if copied_from.trim().is_empty() {
            copied_from
        } else {
            resolve_model_reference(&copied_from, model_dir)?
        };

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
        model_dir: &Path,
    ) -> Result<Self> {
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
            let resolved: Vec<String> = based_on
                .into_iter()
                .map(|b| resolve_model_reference(&b, model_dir))
                .collect::<Result<_>>()?;
            self.based_on = resolved;
        }
        if let Some(c) = copied_from
            && !c.trim().is_empty()
        {
            self.copied_from = resolve_model_reference(&c, model_dir)?;
        }
        Ok(self)
    }

    pub fn update(
        mut self,
        description: Option<String>,
        tags: Vec<String>,
        based_on: Vec<String>,
        model_dir: &Path,
    ) -> Result<Self> {
        // Append mode: merge with existing
        for tag in tags {
            if !self.tags.contains(&tag) {
                self.tags.push(tag)
            }
        }
        for based in based_on {
            let resolved = resolve_model_reference(&based, model_dir)?;
            if !self.based_on.contains(&resolved) {
                self.based_on.push(resolved)
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

        Ok(self)
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

/// Compute a relative path string from `base_dir` to `target`.
///
/// Both inputs must already be canonicalized. The result is a forward-slash-separated
/// relative path such that `base_dir.join(result)` resolves to `target`.
fn relative_from(target: &Path, base_dir: &Path) -> Result<String> {
    let mut base_components: Vec<_> = base_dir.components().collect();
    let mut target_components: Vec<_> = target.components().collect();

    let common_len = base_components
        .iter()
        .zip(target_components.iter())
        .take_while(|(a, b)| a == b)
        .count();
    base_components.drain(..common_len);
    target_components.drain(..common_len);

    let mut parts: Vec<String> = base_components.iter().map(|_| "..".to_string()).collect();
    for c in &target_components {
        parts.push(c.as_os_str().to_string_lossy().into_owned());
    }

    if parts.is_empty() {
        bail!(
            "target and base_dir are the same path: {}",
            target.display()
        );
    }

    Ok(parts.join("/"))
}

fn resolve_model_reference(rel: &str, model_dir: &Path) -> Result<String> {
    let cwd = std::env::current_dir()?;
    resolve_model_reference_with_cwd(rel, model_dir, &cwd)
}

fn resolve_model_reference_with_cwd(rel: &str, model_dir: &Path, cwd: &Path) -> Result<String> {
    let rel_path = Path::new(rel);
    if rel_path.is_absolute() {
        return absolute_to_model_dir_relative(rel_path, model_dir);
    }
    if rel_path.extension().is_some() {
        if model_dir.join(rel_path).exists() {
            return Ok(rel.to_string());
        }
        let cwd_anchored = cwd.join(rel_path);
        if cwd_anchored.exists() {
            return absolute_to_model_dir_relative(&cwd_anchored, model_dir);
        }
        bail!(
            "Model file does not exist: {rel} (resolved to {})",
            model_dir.join(rel_path).display()
        );
    }
    let mod_rel = format!("{rel}.mod");
    let ctl_rel = format!("{rel}.ctl");
    match probe_bare_at(&mod_rel, &ctl_rel, model_dir, rel)? {
        Some(s) => return Ok(s),
        None => {}
    }
    match probe_bare_at(&mod_rel, &ctl_rel, cwd, rel)? {
        Some(name_with_ext) => absolute_to_model_dir_relative(&cwd.join(&name_with_ext), model_dir),
        None => bail!(
            "Model file does not exist: {rel} (no {mod_rel} or {ctl_rel} found in {} or {})",
            model_dir.display(),
            cwd.display()
        ),
    }
}

/// Probe `{anchor}/{mod_rel}` and `{anchor}/{ctl_rel}` for an extensionless reference.
/// Returns `Ok(Some(name_with_ext))` if exactly one exists, `Ok(None)` if neither,
/// `Err` if both (ambiguous).
fn probe_bare_at(mod_rel: &str, ctl_rel: &str, anchor: &Path, rel: &str) -> Result<Option<String>> {
    let mod_exists = anchor.join(mod_rel).exists();
    let ctl_exists = anchor.join(ctl_rel).exists();
    match (mod_exists, ctl_exists) {
        (true, true) => bail!(
            "Ambiguous model reference '{rel}': both {mod_rel} and {ctl_rel} exist. \
             Specify the extension explicitly."
        ),
        (true, false) => Ok(Some(mod_rel.to_string())),
        (false, true) => Ok(Some(ctl_rel.to_string())),
        (false, false) => Ok(None),
    }
}

fn absolute_to_model_dir_relative(target: &Path, model_dir: &Path) -> Result<String> {
    let target = target.canonicalize().map_err(|e| {
        anyhow!(
            "Failed to canonicalize model path '{}': {e}",
            target.display()
        )
    })?;
    let base = model_dir.canonicalize().map_err(|e| {
        anyhow!(
            "Failed to canonicalize model directory '{}': {e}",
            model_dir.display()
        )
    })?;
    relative_from(&target, &base)
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

    let metadata = if metadata_path.exists() {
        let m = ModelMetadata::load(&metadata_path)?;
        if overwrite {
            m.set(description, tags_vec, based_on_vec, copied_from, &model_dir)?
        } else {
            m.update(description, tags_vec, based_on_vec, &model_dir)?
        }
    } else {
        let mut m = ModelMetadata::new(
            based_on_vec,
            copied_from.unwrap_or_default(),
            description.unwrap_or_default(),
            &model_dir,
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
        assert!(ModelMetadata::new(vec![], String::new(), String::new(), Path::new("")).is_err());
        assert!(ModelMetadata::new(vec![], String::new(), "   ".into(), Path::new("")).is_err());
    }

    #[test]
    fn save_and_load_round_trip() {
        let tmp = TempDir::new().unwrap();
        touch(tmp.path(), "base.mod");
        touch(tmp.path(), "src.mod");
        let m = ModelMetadata::new(
            vec!["base.mod".into()],
            "src.mod".into(),
            "desc".into(),
            tmp.path(),
        )
        .unwrap();
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
    fn resolve_model_path_ambiguous_siblings() {
        let tmp = TempDir::new().unwrap();
        touch(tmp.path(), "1010_metadata.json");
        touch(tmp.path(), "1010.mod");
        touch(tmp.path(), "1010.ctl");

        let result = resolve_model_path(tmp.path().join("1010_metadata.json"));
        let err = format!("{}", result.unwrap_err()).to_lowercase();
        assert!(err.contains("1010.mod"), "missing '1010.mod' in '{err}'");
        assert!(err.contains("1010.ctl"), "missing '1010.ctl' in '{err}'");
        assert!(err.contains("ambig"), "missing 'ambig' in '{err}'");
    }

    #[test]
    fn resolve_model_path_single_extension() {
        let tmp = TempDir::new().unwrap();
        touch(tmp.path(), "1010_metadata.json");
        touch(tmp.path(), "1010.mod");

        let result = resolve_model_path(tmp.path().join("1010_metadata.json")).unwrap();
        assert_eq!(result, tmp.path().join("1010.mod"));

        let tmp2 = TempDir::new().unwrap();
        touch(tmp2.path(), "1010_metadata.json");
        touch(tmp2.path(), "1010.ctl");

        let result2 = resolve_model_path(tmp2.path().join("1010_metadata.json")).unwrap();
        assert_eq!(result2, tmp2.path().join("1010.ctl"));
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

    #[test]
    fn resolver_handles_absolute_path() {
        let tmp = TempDir::new().unwrap();
        let src_dir = tmp.path().join("src");
        let dst_dir = tmp.path().join("dst");
        fs::create_dir_all(&src_dir).unwrap();
        fs::create_dir_all(&dst_dir).unwrap();

        let src_model = src_dir.join("parent.mod");
        touch_rel(tmp.path(), "src/parent.mod");

        let abs_path = src_model.to_string_lossy().into_owned();
        let result = resolve_model_reference(&abs_path, &dst_dir).unwrap();

        let resolved = dst_dir.join(&result).canonicalize().unwrap();
        assert_eq!(resolved, src_model.canonicalize().unwrap());
    }

    /// CWD-fallback resolver behavior: when a reference doesn't match anything in `model_dir`,
    /// the resolver tries the user's CWD. Each case sets up files at CWD only (model_dir empty),
    /// and asserts either that the result joins back to the CWD-anchored target file, or that
    /// resolution fails with the right error substrings.
    #[test]
    fn resolver_cwd_fallback_scenarios() {
        enum Expect {
            ResolvesTo(&'static str),
            Fails(&'static [&'static str]),
        }

        struct Case {
            name: &'static str,
            cwd_files: &'static [&'static str],
            input: &'static str,
            expect: Expect,
        }

        let cases = &[
            Case {
                name: "extension_path_with_subdir",
                cwd_files: &["tmp/struct/1001.mod"],
                input: "tmp/struct/1001.mod",
                expect: Expect::ResolvesTo("tmp/struct/1001.mod"),
            },
            Case {
                name: "bare_name_resolves_to_mod",
                cwd_files: &["1010a.mod"],
                input: "1010a",
                expect: Expect::ResolvesTo("1010a.mod"),
            },
            Case {
                name: "bare_name_resolves_to_ctl",
                cwd_files: &["1010a.ctl"],
                input: "1010a",
                expect: Expect::ResolvesTo("1010a.ctl"),
            },
            Case {
                name: "bare_name_with_subdir",
                cwd_files: &["tmp/struct/1002.mod"],
                input: "tmp/struct/1002",
                expect: Expect::ResolvesTo("tmp/struct/1002.mod"),
            },
            Case {
                name: "bare_ambiguous_at_cwd",
                cwd_files: &["1010a.mod", "1010a.ctl"],
                input: "1010a",
                expect: Expect::Fails(&["1010a.mod", "1010a.ctl", "ambig"]),
            },
            Case {
                name: "extension_path_nowhere",
                cwd_files: &[],
                input: "parent.mod",
                expect: Expect::Fails(&["parent.mod"]),
            },
            Case {
                name: "bare_name_nowhere",
                cwd_files: &[],
                input: "1010a",
                expect: Expect::Fails(&["1010a"]),
            },
        ];

        for case in cases {
            let tmp = TempDir::new().unwrap();
            let user_cwd = tmp.path();
            let model_dir = user_cwd.join("model_dir");
            fs::create_dir_all(&model_dir).unwrap();

            for f in case.cwd_files {
                touch_rel(user_cwd, f);
            }

            let result = resolve_model_reference_with_cwd(case.input, &model_dir, user_cwd);
            let label = format!("[{}]", case.name);

            match &case.expect {
                Expect::ResolvesTo(target_rel) => {
                    let s = result.unwrap_or_else(|e| panic!("{label} expected Ok, got Err: {e}"));
                    let resolved = model_dir.join(&s).canonicalize().unwrap();
                    let expected = user_cwd.join(target_rel).canonicalize().unwrap();
                    assert_eq!(
                        resolved, expected,
                        "{label} resolution mismatch (got string: {s})"
                    );
                }
                Expect::Fails(needles) => {
                    let err = match result {
                        Ok(s) => panic!("{label} expected Err, got Ok({s})"),
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

    #[test]
    fn copy_model_cross_directory_metadata_resolves() {
        use crate::copy::{CopyOptions, copy_model};

        static MINIMAL_MODEL: &str = "\
$PROBLEM test
$INPUT ID TIME DV
$DATA data.csv
$THETA 1
";

        let tmp = TempDir::new().unwrap();
        let src_dir = tmp.path().join("src");
        let dst_dir = tmp.path().join("dst");
        fs::create_dir_all(&src_dir).unwrap();
        fs::create_dir_all(&dst_dir).unwrap();

        let src_model = src_dir.join("parent.mod");
        let dst_model = dst_dir.join("child.mod");
        fs::write(&src_model, MINIMAL_MODEL).unwrap();

        let options = CopyOptions {
            description: "cross-dir".into(),
            ..Default::default()
        };

        copy_model(&src_model, &dst_model, "parent.mod", "child.mod", &options).unwrap();

        let m = read_metadata(&dst_dir, "child");
        let resolved = dst_dir.join(&m.copied_from).canonicalize().unwrap();
        assert_eq!(resolved, src_model.canonicalize().unwrap());
    }
}
