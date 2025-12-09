use anyhow::{Result as AnyhowResult, bail};
use serde::{Deserialize, Serialize};
use statrs::distribution::{ContinuousCDF, Normal};

use crate::output_files::ext::ParameterType;

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

    pub fn compute_rse(&self, estimate: f64, se: f64, param_type: &ParameterType) -> f64 {
        use ParameterType as P;
        use Transform as T;

        match param_type {
            P::Theta => match self {
                T::LogNormal => (se.powi(2).exp() - 1.0).sqrt() * 100.0,
                _ => se / estimate.abs() * 100.0,
            },
            P::Omega | P::Sigma => (estimate.powi(2).exp() - 1.0).sqrt() * 100.0,
        }
    }

    pub fn compute_cv(&self, estimate: f64, param_type: &ParameterType) -> Option<f64> {
        use ParameterType as P;
        use Transform as T;

        match param_type {
            P::Theta => None,
            P::Omega | P::Sigma => match self {
                T::LogNormal | T::AddErr => Some((estimate.exp() - 1.0).sqrt() * 100.0),
                T::Proportional => Some(estimate.sqrt() * 100.0),
                T::Identity => None,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f64 = 1e-9;

    // ==================== compute_cv tests ====================

    #[test]
    fn test_compute_cv_prop_omega() {
        let t = Transform::Proportional;
        let p = ParameterType::Omega;

        let result = t.compute_cv(0.09, &p).unwrap();

        assert!((result - 30.0).abs() < EPS, "expected 30.0 got {result}");
    }

    #[test]
    fn test_compute_cv_lognorm_omega() {
        let t = Transform::LogNormal;
        let p = ParameterType::Omega;

        let input = (0.6_f64.powi(2) + 1.0).ln();
        let result = t.compute_cv(input, &p).unwrap();

        assert!((result - 60.0).abs() < EPS, "expected ~60.0, got {result}");
    }

    #[test]
    fn test_compute_cv_adderr_sigma() {
        let t = Transform::AddErr;
        let p = ParameterType::Sigma;

        let input = (0.5_f64.powi(2) + 1.0).ln();
        let result = t.compute_cv(input, &p).unwrap();

        assert!((result - 50.0).abs() < EPS, "expected ~50.0, got {result}");
    }

    #[test]
    fn test_compute_cv_theta_returns_none() {
        let t = Transform::LogNormal;
        let p = ParameterType::Theta;

        assert!(t.compute_cv(0.5, &p).is_none());
    }

    #[test]
    fn test_compute_cv_identity_returns_none() {
        let t = Transform::Identity;
        let p = ParameterType::Omega;

        assert!(t.compute_cv(0.5, &p).is_none());
    }

    // ==================== compute_rse tests ====================

    #[test]
    fn test_compute_rse_identity_theta() {
        let t = Transform::Identity;
        let p = ParameterType::Theta;

        let result = t.compute_rse(10.0, 2.0, &p).unwrap();

        assert!((result - 20.0).abs() < EPS, "expected 20.0, got {result}");
    }

    #[test]
    fn test_compute_rse_lognorm_theta() {
        let t = Transform::LogNormal;
        let p = ParameterType::Theta;

        let se: f64 = 0.1;
        let expected = (se.powi(2).exp() - 1.0).sqrt() * 100.0;
        let result = t.compute_rse(1.0, se, &p).unwrap();

        assert!(
            (result - expected).abs() < EPS,
            "expected {expected}, got {result}"
        );
    }

    #[test]
    fn test_compute_rse_omega_returns_none() {
        let t = Transform::LogNormal;
        let p = ParameterType::Omega;

        assert!(t.compute_rse(0.5, 0.1, &p).is_none());
    }

    #[test]
    fn test_compute_rse_sigma_returns_none() {
        let t = Transform::Identity;
        let p = ParameterType::Sigma;

        assert!(t.compute_rse(0.5, 0.1, &p).is_none());
    }

    // ==================== compute_ci tests ====================

    #[test]
    fn test_compute_ci_identity_95() {
        let t = Transform::Identity;

        let (lower, upper) = t.compute_ci(10.0, 2.0, 0.95).unwrap();

        // 95% CI: estimate ± 1.96 * se
        let expected_lower = 10.0 - 1.96 * 2.0;
        let expected_upper = 10.0 + 1.96 * 2.0;

        assert!(
            (lower - expected_lower).abs() < 0.01,
            "expected {expected_lower}, got {lower}"
        );
        assert!(
            (upper - expected_upper).abs() < 0.01,
            "expected {expected_upper}, got {upper}"
        );
    }

    #[test]
    fn test_compute_ci_lognormal_back_transforms() {
        let t = Transform::LogNormal;

        let estimate = 2.0_f64.ln(); // log(2)
        let se = 0.1;
        let (lower, upper) = t.compute_ci(estimate, se, 0.95).unwrap();

        // Should be back-transformed (exponentiated)
        assert!(lower > 0.0 && lower < 2.0);
        assert!(upper > 2.0);
    }
}
