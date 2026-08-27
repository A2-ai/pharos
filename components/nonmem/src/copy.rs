use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use anyhow::{Result, bail};
#[cfg(feature = "cli")]
use clap::Parser;
use fs_err as fs;
use nonmem_parser::Model;
use serde::{Deserialize, Serialize};

use crate::ModelMetadata;

#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize, Hash, Eq)]
#[serde(rename_all = "lowercase")]
pub enum UpdateType {
    All,
    None,
    Theta,
    Omega,
    Sigma,
}

impl FromStr for UpdateType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "all" => Ok(UpdateType::All),
            "none" => Ok(UpdateType::None),
            "theta" | "thetas" => Ok(UpdateType::Theta),
            "omega" | "omegas" => Ok(UpdateType::Omega),
            "sigma" | "sigmas" => Ok(UpdateType::Sigma),
            _ => Err(format!("Unknown update type: {}", s)),
        }
    }
}

#[cfg(feature = "cli")]
fn parse_jitter_spec(s: &str) -> Result<f64, String> {
    let percentage = s
        .parse::<f64>()
        .map_err(|_| format!("Invalid percentage value: '{}'", s))?;

    if !(0.0..=1.0).contains(&percentage) {
        return Err(format!(
            "Jitter percentage must be between 0.0 and 1.0, got {}",
            percentage
        ));
    }

    Ok(percentage)
}

#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
#[cfg_attr(feature = "cli", derive(Parser))]
pub struct CopyOptions {
    /// What to update: all, none, theta, omega, sigma (can be combined)
    ///
    /// Note: 'all' and 'none' cannot be combined with other values
    ///
    /// Examples: --update all, --update theta,omega, --update none
    ///
    /// Defaults to "none"
    #[cfg_attr(
        feature = "cli",
        clap(long, value_delimiter = ',', default_value = "none")
    )]
    pub update: Vec<UpdateType>,

    /// Path to the .ext file containing parameter estimates to use.
    ///
    /// If not specified, it will try {model_name}/{model_name}.ext and the output_dir defined
    /// in the config.
    #[cfg_attr(feature = "cli", clap(long))]
    pub ext_path: Option<PathBuf>,

    /// Jitter percentage for THETA parameters
    ///
    /// You can use jitter even if --update=none, in which case it will jitter the initial values
    /// Example: --jitter 0.2
    #[cfg_attr(feature = "cli", clap(
        long,
        value_parser = parse_jitter_spec
    ))]
    pub jitter: Option<f64>,

    /// Random seed for reproducible jittering
    #[cfg_attr(feature = "cli", clap(long))]
    pub seed: Option<u64>,

    /// Exclude specific parameters from jittering (comma-separated, e.g. "THETA1,THETA2")
    #[cfg_attr(feature = "cli", clap(long))]
    pub jitter_excluded: Option<String>,

    /// A description to add to the metadata file
    #[cfg_attr(feature = "cli", clap(long))]
    pub description: String,

    /// Determines model hierarchy. Only use for nested models.
    #[cfg_attr(feature = "cli", clap(long, value_delimiter = ','))]
    pub based_on: Vec<String>,

    /// Tags to attach to the metadata for the copied model
    #[cfg_attr(feature = "cli", clap(long, value_delimiter = ','))]
    pub tags: Vec<String>,

    #[cfg_attr(feature = "cli", clap(long))]
    pub no_metadata: bool,
}

impl CopyOptions {
    /// Validate the update configuration
    pub fn validate_update(&self) -> Result<(), String> {
        let unique_updates: HashSet<UpdateType> = self.update.iter().cloned().collect();

        if unique_updates.contains(&UpdateType::None) && unique_updates.len() > 1 {
            return Err("'none' cannot be combined with other update types".to_string());
        }

        if unique_updates.contains(&UpdateType::All) && unique_updates.len() > 1 {
            return Err("'all' cannot be combined with other update types".to_string());
        }

        Ok(())
    }

    /// Whether we want to update params from the final estimates
    pub fn is_updating_params(&self) -> bool {
        self.update != vec![UpdateType::None]
    }

    pub fn has_jittering(&self) -> bool {
        self.jitter.is_some()
    }

    fn param_update(&self, update_type: UpdateType) -> bool {
        self.update.contains(&update_type) || self.update.contains(&UpdateType::All)
    }

    pub fn theta_updates(&self) -> bool {
        self.param_update(UpdateType::Theta)
    }

    pub fn omega_updates(&self) -> bool {
        self.param_update(UpdateType::Omega)
    }

    pub fn sigma_updates(&self) -> bool {
        self.param_update(UpdateType::Sigma)
    }

    pub fn excluded_parameters(&self) -> Vec<String> {
        self.jitter_excluded
            .as_ref()
            .map(|s| {
                let mut params = Vec::new();
                let mut current = String::new();
                let mut paren_depth = 0;

                for ch in s.to_ascii_uppercase().chars() {
                    match ch {
                        '(' => {
                            paren_depth += 1;
                            current.push(ch);
                        }
                        ')' => {
                            paren_depth -= 1;
                            current.push(ch);
                        }
                        ',' if paren_depth == 0 => {
                            if !current.trim().is_empty() {
                                params.push(current.trim().to_string());
                            }
                            current.clear();
                        }
                        _ => current.push(ch),
                    }
                }

                if !current.trim().is_empty() {
                    params.push(current.trim().to_string());
                }

                params
            })
            .unwrap_or_default()
    }
}

/// Read parameter estimates from .ext file and build a HashMap keyed by parameter name.
/// Only includes the parameter types specified by the options.
fn read_estimates(options: &CopyOptions) -> Result<HashMap<String, f64>> {
    let Some(ext_path) = &options.ext_path else {
        return Ok(HashMap::new());
    };

    // Strict read: only the final-estimates row counts; a missing value comes
    // back as NaN so we can reject it with context below.
    let estimates = crate::update::read_ext_estimates(ext_path, &options.update, false)?;

    // If we can't parse the value row, we will put NaN instead as value.
    // This can happen if we're trying to copy a run that hasn't finished yet or has some
    // garbage
    let excluded = options.excluded_parameters();
    for (name, value) in &estimates {
        if !value.is_finite() && !excluded.iter().any(|e| e == name) {
            bail!(
                "Invalid estimate found for {name} in {}, the run may not have finished.",
                ext_path.display()
            );
        }
    }

    Ok(estimates)
}

pub fn copy_model(
    from: &Path,
    to: &Path,
    original_filename: &str,
    new_filename: &str,
    options: &CopyOptions,
) -> Result<()> {
    let from_model = Model::parse(from, &fs::read_to_string(from)?)?;
    log::debug!("Copying model from {from:?} to {to:?} with options {options:?}");
    let mut new_model = from_model.copy(original_filename, new_filename);

    // Update initial estimates if requested
    if options.is_updating_params() || options.has_jittering() {
        log::debug!("Updating {to:?} parameters");
        let estimates = if options.is_updating_params() {
            read_estimates(options)?
        } else {
            HashMap::new()
        };
        let excluded = options.excluded_parameters();
        new_model.update_initial_estimates(&estimates, options.jitter, options.seed, &excluded);
    }

    let new_model_name = to.file_stem().unwrap().to_string_lossy();

    // A relative $DATA path is resolved relative to the model's own directory, so
    // moving the model to a different directory would break it. Rewrite it to point
    // at the same dataset from the new location. Absolute paths are left untouched.
    if Path::new(&new_model.data.path).is_relative() {
        let from_dir = from.parent().unwrap_or(Path::new("."));
        let to_dir = to.parent().unwrap_or(Path::new("."));
        if let Some(new_data_path) = rebase_relative_path(&new_model.data.path, from_dir, to_dir) {
            new_model.update_data_path(&new_data_path);
        }
    }

    // Ensure the destination directory exists so metadata and model writes succeed
    if let Some(parent) = to.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }

    // Create metadata file
    if !options.no_metadata {
        let model_dir = to
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let from_canonical = fs::canonicalize(from)?;
        let metadata = ModelMetadata::new(
            options.based_on.clone(),
            from_canonical.to_string_lossy().into_owned(),
            options.description.clone(),
            options.tags.clone(),
            model_dir,
        )?;
        metadata.save(new_model_name.as_ref(), model_dir)?;
    }

    // Saving model file after metadata is created in case description not provided
    // and re-running copy fails due to no --overwrite
    let mut f = fs::File::create(to)?;
    f.write_all(new_model.model_content().as_bytes())?;

    Ok(())
}

/// Re-express a relative path written against `from_dir` so that it resolves to the
/// same target from `to_dir`. Returns `None` if the result equals the input (no move).
/// Purely lexical — does not touch the filesystem, so the dataset need not exist.
fn rebase_relative_path(rel_path: &str, from_dir: &Path, to_dir: &Path) -> Option<String> {
    let cwd = std::env::current_dir().ok()?;
    let target = normalize_path(&cwd.join(from_dir).join(rel_path));
    let to_abs = normalize_path(&cwd.join(to_dir));
    let new_rel = relative_path_from(&target, &to_abs);
    if new_rel == rel_path {
        None
    } else {
        Some(new_rel)
    }
}

/// Lexically resolve `.` and `..` components without consulting the filesystem.
fn normalize_path(path: &Path) -> PathBuf {
    use std::path::Component;
    let mut out = PathBuf::new();
    for comp in path.components() {
        match comp {
            Component::CurDir => {}
            Component::ParentDir => match out.components().next_back() {
                // Collapse against a directory we actually descended into.
                Some(Component::Normal(_)) => {
                    out.pop();
                }
                // At an absolute root (or drive prefix) `..` has nowhere to go, so
                // clamp — matching how the OS resolves it — rather than emit `/..`.
                Some(Component::RootDir | Component::Prefix(_)) => {}
                // Relative path with no parent to pop: keep `..` to preserve depth.
                _ => out.push(".."),
            },
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// Express `target` relative to `base`, assuming both are normalized absolute paths.
/// Components are joined with `/` (not the OS separator) so the result stays portable
/// inside the model file regardless of the platform the copy runs on.
fn relative_path_from(target: &Path, base: &Path) -> String {
    let target_comps: Vec<_> = target.components().collect();
    let base_comps: Vec<_> = base.components().collect();
    let common = target_comps
        .iter()
        .zip(&base_comps)
        .take_while(|(a, b)| a == b)
        .count();

    let mut parts: Vec<String> = Vec::new();
    for _ in common..base_comps.len() {
        parts.push("..".to_string());
    }
    for comp in &target_comps[common..] {
        parts.push(comp.as_os_str().to_string_lossy().into_owned());
    }
    parts.join("/")
}

#[cfg(test)]
mod tests {
    use super::*;

    // Absolute dirs so the result is independent of the test's working directory.
    fn rebase(rel: &str, from: &str, to: &str) -> Option<String> {
        rebase_relative_path(rel, Path::new(from), Path::new(to))
    }

    #[test]
    fn rebase_relative_path_cases() {
        let cases = [
            // Sibling dirs at the same depth resolve to the same place, so no rebase.
            (
                "../../data/pk.csv",
                "/proj/models/onecmt",
                "/proj/models/twocmt",
                None,
            ),
            // Dataset in the model dir must point back to the original dir.
            (
                "pk.csv",
                "/proj/models/onecmt",
                "/proj/models/twocmt",
                Some("../onecmt/pk.csv"),
            ),
            // Deeper destination adds parent segments.
            (
                "../data/pk.csv",
                "/proj/models",
                "/proj/models/a/b/c",
                Some("../../../../data/pk.csv"),
            ),
        ];

        for (rel, from, to, expected) in cases {
            assert_eq!(
                rebase(rel, from, to),
                expected.map(str::to_string),
                "rebase({rel:?}, {from:?}, {to:?})"
            );
        }
    }

    #[test]
    fn read_estimates_rejects_unfinished_run() {
        // still_running.ext has no -1000000000 final-estimates row, so every
        // parameter comes back NaN. We must refuse rather than write NaN into
        // the child model.
        let opts = CopyOptions {
            update: vec![UpdateType::All],
            ext_path: Some(
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("test_data")
                    .join("copy/still_running.ext"),
            ),
            ..Default::default()
        };
        let err = read_estimates(&opts).expect_err("an unfinished run should be rejected");
        let msg = err.to_string();
        assert!(
            msg.contains("may not have finished"),
            "unexpected error: {msg}"
        );
    }
}
