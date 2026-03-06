use std::collections::HashSet;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use anyhow::Result;
#[cfg(feature = "cli")]
use clap::Parser;
use fs_err as fs;
use serde::{Deserialize, Serialize};

use crate::{Model, ModelMetadata};

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

    pub fn theta_updates(&self) -> (bool, Option<f64>) {
        (self.param_update(UpdateType::Theta), self.jitter)
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

pub fn copy_model(
    from: &Path,
    to: &Path,
    original_filename: &str,
    new_filename: &str,
    options: &CopyOptions,
) -> Result<()> {
    let from_model = Model::parse(&fs::read_to_string(from)?)?;
    log::debug!("Copying model from {from:?} to {to:?} with options {options:?}");
    let mut new_model = from_model.copy(original_filename, new_filename)?;

    // Update initial estimates if requested
    if options.is_updating_params() || options.has_jittering() {
        log::debug!("Updating {to:?} parameters");
        new_model.update_initial_estimates(options)?;
    }

    let new_model_name = to.file_stem().unwrap().to_string_lossy();

    // Create metadata file
    if !options.no_metadata {
        let metadata = ModelMetadata::new(
            vec![original_filename.to_string()],
            options.description.clone(),
        )?;
        metadata.save(new_model_name.as_ref(), to.parent().unwrap())?;
    }

    // Saving model file after metadata is created in case description not provided
    // and re-running copy fails due to no --overwrite
    let mut f = fs::File::create(to)?;
    f.write_all(new_model.model_content().as_bytes())?;

    Ok(())
}
