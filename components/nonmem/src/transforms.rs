use anyhow::{Result as AnyhowResult, bail};
use serde::{Deserialize, Serialize};
use statrs::distribution::{ContinuousCDF, Normal};

use crate::output_files::ext::ParameterType;

/// Get z-score for confidence level
fn ci_z_score(ci_level: f64) -> AnyhowResult<f64> {
    let normal = Normal::new(0.0, 1.0).unwrap();

    // For a two-tailed CI, we need the quantile at (1 + level) / 2
    // e.g., 95% CI -> quantile at 0.975 -> z ≈ 1.96
    if !(ci_level > 0.0 && ci_level < 1.0) {
        bail!("ci_level must be between 0 and 1 (exclusive), got {ci_level}");
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

        match (param_type, self) {
            // For Theta lognormal RSE we use the SE on the log scale
            (P::Theta, T::LogNormal) => (se.powi(2).exp() - 1.0).sqrt() * 100.0,
            // For Omega/Sigma the stored estimate is a variance term, so use it directly
            (P::Omega | P::Sigma, T::LogNormal) => (estimate.powi(2).exp() - 1.0).sqrt() * 100.0,
            _ => se / estimate.abs() * 100.0,
        }
    }

    pub fn compute_cv(&self, estimate: f64, param_type: &ParameterType) -> Option<f64> {
        use ParameterType as P;
        use Transform as T;

        match param_type {
            P::Theta => None,
            P::Omega => match self {
                T::LogNormal => Some((estimate.exp() - 1.0).sqrt() * 100.0),
                T::Proportional => Some(estimate.sqrt() * 100.0),
                // This would require associated theta parameter to compute.
                T::AddErr | T::Identity => None,
            },
            P::Sigma => match self {
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

    const EPS: f64 = 1e-6;

    // Helper to create estimate that yields a specific CV% for lognormal
    fn lognorm_estimate_for_cv(cv_pct: f64) -> f64 {
        ((cv_pct / 100.0).powi(2) + 1.0).ln()
    }

    #[test]
    fn test_compute_cv() {
        use ParameterType as P;
        use Transform as T;

        let cases: Vec<(T, P, f64, Option<f64>, &str)> = vec![
            // Branch 1: Theta -> None
            (T::LogNormal, P::Theta, 1.0, None, "Theta"),
            // Branch 2: Omega/Sigma + Identity -> None
            (T::Identity, P::Omega, 0.5, None, "Omega/Identity"),
            (T::AddErr, P::Omega, 0.5, None, "Omega/AddErr"),
            // Branch 3: Omega/Sigma + LogNormal|AddErr -> lognormal formula
            (
                T::LogNormal,
                P::Omega,
                lognorm_estimate_for_cv(30.0),
                Some(30.0),
                "Omega/LogNormal",
            ),
            (
                T::AddErr,
                P::Sigma,
                lognorm_estimate_for_cv(40.0),
                Some(40.0),
                "Sigma/AddErr",
            ),
            // Branch 4: Omega/Sigma + Proportional -> sqrt formula
            (
                T::Proportional,
                P::Sigma,
                0.09,
                Some(30.0),
                "Sigma/Proportional",
            ),
            (
                T::Proportional,
                P::Omega,
                0.09,
                Some(30.0),
                "Omega/Proportional",
            ),
            // Branch 5: Sigma + Identity -> None
            (T::Identity, P::Sigma, 0.5, None, "Sigma/Identity"),
        ];

        for (transform, param_type, estimate, expected, name) in cases {
            let result = transform.compute_cv(estimate, &param_type);
            match (result, expected) {
                (None, None) => {}
                (Some(r), Some(e)) => assert!((r - e).abs() < EPS, "{name}: expected {e}, got {r}"),
                (r, e) => panic!("{name}: expected {e:?}, got {r:?}"),
            }
        }
    }

    #[test]
    fn test_compute_rse() {
        use ParameterType as P;
        use Transform as T;

        let cases: Vec<(T, P, f64, f64, f64, &str)> = vec![
            // Branch 1: Theta + LogNormal -> SE-based formula
            (
                T::LogNormal,
                P::Theta,
                1.0,
                0.1,
                (0.1_f64.powi(2).exp() - 1.0).sqrt() * 100.0,
                "Theta/LogNormal",
            ),
            // Branch 2: Omega/Sigma + LogNormal -> estimate-based formula
            (
                T::LogNormal,
                P::Omega,
                0.5,
                0.1,
                (0.5_f64.powi(2).exp() - 1.0).sqrt() * 100.0,
                "Omega/LogNormal",
            ),
            // Branch 3: wildcard -> standard RSE (se / |estimate| * 100)
            (T::Identity, P::Theta, 10.0, 2.0, 20.0, "Theta/Identity"),
            (T::Identity, P::Sigma, 0.5, 0.1, 20.0, "Sigma/Identity"),
        ];

        for (transform, param_type, estimate, se, expected, name) in cases {
            let result = transform.compute_rse(estimate, se, &param_type);
            assert!(
                (result - expected).abs() < EPS,
                "{name}: expected {expected}, got {result}"
            );
        }
    }

    #[test]
    fn test_compute_ci() {
        use Transform as T;

        let z_95 = 1.959964;

        let cases: Vec<(T, f64, f64, f64, f64, &str)> = vec![
            // Branch 1: LogNormal -> back-transform with exp()
            (
                T::LogNormal,
                2.0_f64.ln(),
                0.1,
                (2.0_f64.ln() - z_95 * 0.1).exp(),
                (2.0_f64.ln() + z_95 * 0.1).exp(),
                "LogNormal",
            ),
            // Branch 2: others -> no back-transform
            (
                T::Identity,
                10.0,
                2.0,
                10.0 - z_95 * 2.0,
                10.0 + z_95 * 2.0,
                "Identity",
            ),
        ];

        for (transform, estimate, se, exp_lower, exp_upper, name) in cases {
            let (lower, upper) = transform.compute_ci(estimate, se, 0.95).unwrap();
            assert!(
                (lower - exp_lower).abs() < 0.001,
                "{name}: lower expected {exp_lower}, got {lower}"
            );
            assert!(
                (upper - exp_upper).abs() < 0.001,
                "{name}: upper expected {exp_upper}, got {upper}"
            );
        }
    }

    #[test]
    fn test_compute_ci_invalid_levels() {
        let t = Transform::Identity;
        assert!(t.compute_ci(10.0, 2.0, -0.1).is_err());
        assert!(t.compute_ci(10.0, 2.0, 0.0).is_err());
        assert!(t.compute_ci(10.0, 2.0, 1.0).is_err());
        assert!(t.compute_ci(10.0, 2.0, 1.5).is_err());
    }

    #[test]
    fn test_back_transform() {
        use Transform as T;

        // Branch 1: LogNormal -> exp()
        assert!((T::LogNormal.back_transform(1.0) - 1.0_f64.exp()).abs() < EPS);
        // Branch 2: others -> identity
        assert!((T::Identity.back_transform(5.0) - 5.0).abs() < EPS);
    }

    #[test]
    fn test_from_str() {
        use Transform as T;

        // One per variant
        assert_eq!("identity".parse::<T>().unwrap(), T::Identity);
        assert_eq!("lognormal".parse::<T>().unwrap(), T::LogNormal);
        assert_eq!("proportional".parse::<T>().unwrap(), T::Proportional);
        assert_eq!("adderr".parse::<T>().unwrap(), T::AddErr);
        // Case insensitive
        assert_eq!("LOGNORMAL".parse::<T>().unwrap(), T::LogNormal);
        // Invalid
        assert!("unknown".parse::<T>().is_err());
    }
}
