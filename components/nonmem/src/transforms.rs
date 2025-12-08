use anyhow::{Result as AnyhowResult, bail};
use serde::{Deserialize, Serialize};
use statrs::distribution::{ContinuousCDF, Normal};

/// Get z-score for confidence level
fn ci_z_score(ci_level: f64) -> AnyhowResult<f64> {
    let normal = Normal::new(0.0, 1.0).unwrap();

    // For a two-tailed CI, we need the quantile at (1 + level) / 2
    // e.g., 95% CI -> quantile at 0.975 -> z ≈ 1.96
    if !(0.0..=1.0).contains(&ci_level) {
        bail!("ci_level must be between 0 and 1, got {ci_level}");
    }

    Ok(normal.inverse_cdf((1.0 + ci_level) / 2.0))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Transform {
    Identity,
    LogNormal,
    Proportional,
    AddErr,
}

impl std::str::FromStr for Transform {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "identity" => Ok(Transform::Identity),
            "lognormal" | "log_normal" => Ok(Transform::LogNormal),
            "proportional" => Ok(Transform::Proportional),
            "adderr" | "additive" => Ok(Transform::AddErr),
            _ => bail!("Unknown transform: {}", s),
        }
    }
}

impl Transform {
    pub fn back_transform(&self, value: f64) -> f64 {
        use Transform as T;

        match self {
            T::LogNormal => value.exp(),
            T::Proportional | T::AddErr | T::Identity => value,
        }
    }

    // Meaningful for Theta/Omega/Sigma unfixed
    pub fn compute_ci(&self, estimate: f64, se: f64, ci_level: f64) -> AnyhowResult<(f64, f64)> {
        let z = ci_z_score(ci_level)?;
        Ok((
            self.back_transform(estimate - z * se),
            self.back_transform(estimate + z * se),
        ))
    }

    // Meaningful for Theta
    pub fn compute_rse(&self, estimate: f64, se: f64) -> f64 {
        use Transform as T;
        match self {
            T::LogNormal => (se.powi(2).exp() - 1.0).sqrt() * 100.0,
            _ => se / estimate.abs() * 100.0,
        }
    }

    // Meangingful for Omega/Sigma Diagonal parameters
    pub fn compute_cv(&self, estimate: f64) -> Option<f64> {
        use Transform as T;
        match self {
            T::LogNormal | T::AddErr => Some((estimate.exp() - 1.0).sqrt() * 100.0),
            T::Proportional => Some(estimate.sqrt() * 100.0),
            T::Identity => None,
        }
    }
}
