use anyhow::{Result as AnyhowResult, bail};
use distrs::Normal;
use serde::{Deserialize, Serialize};

use crate::output_files::ext::ParameterType;

/// Get z-score for confidence level
fn ci_z_score(ci_level: f64) -> AnyhowResult<f64> {
    // For a two-tailed CI, we need the quantile at (1 + level) / 2
    // e.g., 95% CI -> quantile at 0.975 -> z ≈ 1.96
    if !(ci_level > 0.0 && ci_level < 1.0) {
        bail!("ci_level must be between 0 and 1 (exclusive), got {ci_level}");
    }
    let p = (1.0 + ci_level) / 2.0;
    Ok(Normal::ppf(p, 0.0, 1.0))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Transform {
    // do nothing transform
    Identity,
    // For lognormally distributed parameters -
    // mu referenced thetas EXP(THETA(1) + ETA(1)),
    // and EXP(ETA(i))s
    LogNormal,
    // For logit transformed thetas
    Logit,
    // For proportional etas and eps
    // THETA(i) * (1 + ETA(i)) or
    // Y = F*(1 + EPS(1))
    Proportional,
    // Err for error terms either sigmas or thetas
    // Y = F + THETA(x) * EPS(1)
    AddErr,
    // Err for error terms from sigma or theta
    // Y = LOG(F) + THETA(X) * EPS(1)
    LogAddErr,
}

impl std::str::FromStr for Transform {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "identity" => Ok(Transform::Identity),
            "lognormal" | "log_normal" => Ok(Transform::LogNormal),
            "logit" | "log_it" => Ok(Transform::Logit),
            "proportional" => Ok(Transform::Proportional),
            "adderr" | "additive" => Ok(Transform::AddErr),
            "logadderr" | "logadd" => Ok(Transform::LogAddErr),
            _ => bail!("Unknown transform: {}", s),
        }
    }
}

impl Default for Transform {
    fn default() -> Self {
        Transform::Identity
    }
}

impl Transform {
    /// Transforms value to relevant scale
    /// LogNormal -> exp(value)
    /// otherwise -> value
    pub fn back_transform(&self, value: f64) -> f64 {
        use Transform as T;

        match self {
            T::LogNormal => value.exp(),
            T::Logit => 1.0 / (1.0 + (-value).exp()),
            T::Identity | T::Proportional | T::AddErr | T::LogAddErr => value,
        }
    }

    /// Computes Confidence Intervals and back transforms them.
    /// Errors when ci_level is outside of (0, 1).
    pub fn compute_ci(&self, estimate: f64, se: f64, ci_level: f64) -> AnyhowResult<(f64, f64)> {
        let z = ci_z_score(ci_level)?;
        Ok((
            self.back_transform(estimate - z * se),
            self.back_transform(estimate + z * se),
        ))
    }

    /// Computes percent relative standard error
    pub fn compute_rse(&self, estimate: f64, se: f64, param_type: &ParameterType) -> f64 {
        use ParameterType as P;
        use Transform as T;

        match (param_type, self) {
            // For Theta lognormal RSE we use the SE on the log scale
            (P::Theta, T::LogNormal) => (se.powi(2).exp() - 1.0).sqrt() * 100.0,
            (P::Theta, T::Logit) => (1.0 - self.back_transform(estimate)) * se * 100.0,
            _ => se / estimate.abs() * 100.0,
        }
    }

    /// Computes percent Coefficient of Variation.
    /// Returns None when CV is not meaningful for the parameter/transform combination.
    pub fn compute_cv(&self, estimate: f64, param_type: &ParameterType) -> Option<f64> {
        use ParameterType as P;
        use Transform as T;

        match param_type {
            P::Theta => match self {
                T::LogAddErr => Some((estimate.powi(2).exp() - 1.0).sqrt() * 100.0),
                _ => None,
            },
            P::Omega => match self {
                T::LogNormal => Some((estimate.exp() - 1.0).sqrt() * 100.0),
                T::Proportional => Some(estimate.sqrt() * 100.0),
                _ => None,
            },
            P::Sigma => match self {
                T::LogNormal | T::LogAddErr => Some((estimate.exp() - 1.0).sqrt() * 100.0),
                T::Proportional => Some(estimate.sqrt() * 100.0),
                _ => None,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f64 = 1e-6;

    // Helper to create estimate that yields a specific CV% for lognormal
    // For Omega/Sigma where estimate is variance on log scale
    fn lognorm_estimate_for_cv(cv_pct: f64) -> f64 {
        ((cv_pct / 100.0).powi(2) + 1.0).ln()
    }

    // Helper for Theta/LogAddErr where estimate is SD (gets squared in formula)
    fn logadderr_theta_estimate_for_cv(cv_pct: f64) -> f64 {
        ((cv_pct / 100.0).powi(2) + 1.0).ln().sqrt()
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
            // Branch 3: Omega/Sigma + LogNormal
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
                None,
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
            // Branch 6: Omega/Sigma + Logit -> None
            (T::Logit, P::Omega, 0.5, None, "Omega/Logit"),
            (T::Logit, P::Sigma, 0.5, None, "Sigma/Logit"),
            // Branch 7: Sigma + LogNormal
            (
                T::LogNormal,
                P::Sigma,
                lognorm_estimate_for_cv(40.0),
                Some(40.0),
                "Sigma/LogNormal",
            ),
            // Branch 8: LogAddErr
            (
                T::LogAddErr,
                P::Theta,
                logadderr_theta_estimate_for_cv(30.0),
                Some(30.0),
                "Theta/LogAddErr",
            ),
            (T::LogAddErr, P::Omega, 0.5, None, "Omega/LogAddErr"),
            (
                T::LogAddErr,
                P::Sigma,
                lognorm_estimate_for_cv(40.0),
                Some(40.0),
                "Sigma/LogAddErr",
            ),
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
            // Branch 2: Theta + Logit -> delta method formula
            // back_transform(0.0) = 0.5, so RSE = (1 - 0.5) * 0.1 * 100 = 5.0
            (T::Logit, P::Theta, 0.0, 0.1, 5.0, "Theta/Logit"),
            // Branch 3: Omega/Sigma + LogNormal
            (
                T::LogNormal,
                P::Omega,
                0.5,
                0.1,
                0.1 / 0.5 * 100.0,
                "Omega/LogNormal",
            ),
            // Branch 4: wildcard -> standard RSE (se / |estimate| * 100)
            (T::Identity, P::Theta, 10.0, 2.0, 20.0, "Theta/Identity"),
            (T::Identity, P::Sigma, 0.5, 0.1, 20.0, "Sigma/Identity"),
            // LogAddErr falls through to wildcard
            (T::LogAddErr, P::Theta, 0.5, 0.1, 20.0, "Theta/LogAddErr"),
            (T::LogAddErr, P::Sigma, 0.5, 0.1, 20.0, "Sigma/LogAddErr"),
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
        let z_90 = 1.6449;

        // (transform, estimate, se, ci_level, exp_lower, exp_upper)
        let cases: Vec<(T, f64, f64, f64, f64, f64)> = vec![
            (
                T::LogNormal,
                2.0_f64.ln(),
                0.1,
                0.95,
                (2.0_f64.ln() - z_95 * 0.1).exp(),
                (2.0_f64.ln() + z_95 * 0.1).exp(),
            ),
            (
                T::Identity,
                10.0,
                2.0,
                0.90,
                10.0 - z_90 * 2.0,
                10.0 + z_90 * 2.0,
            ),
        ];

        for (t, estimate, se, ci_level, exp_lower, exp_upper) in cases {
            let (lower, upper) = t.compute_ci(estimate, se, ci_level).unwrap();
            assert!((lower - exp_lower).abs() < 0.001);
            assert!((upper - exp_upper).abs() < 0.001);
        }

        // Invalid ci_level values
        let t = T::Identity;
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
}
