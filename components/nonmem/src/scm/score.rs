use serde::{Deserialize, Serialize};
use statrs::distribution::{ChiSquared, ContinuousCDF};

use super::Direction;

pub fn chi2_sf(x: f64, df: usize) -> f64 {
    if x <= 0.0 || df == 0 {
        return 1.0;
    }
    ChiSquared::new(df as f64)
        .map(|dist| dist.sf(x))
        .unwrap_or(1.0)
}

/// Inverse of [`chi2_sf`]: the statistic `x` with P(X > x) = `p` for X ~ chi2(df)
pub fn chi2_isf(p: f64, df: usize) -> f64 {
    if !(p > 0.0 && p < 1.0) || df == 0 {
        return f64::NAN;
    }
    ChiSquared::new(df as f64)
        .map(|dist| dist.inverse_cdf(1.0 - p))
        .unwrap_or(f64::NAN)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LrtResult {
    pub delta_ofv: f64,
    pub statistic: f64,
    pub df: usize,
    pub p_value: f64,
}

impl Direction {
    pub fn statistic(self, delta_ofv: f64) -> f64 {
        match self {
            Direction::Forward => (-delta_ofv).max(0.0),
            Direction::Backward => delta_ofv.max(0.0),
        }
    }

    pub fn meets(self, p_value: f64, alpha: f64) -> bool {
        match self {
            Direction::Forward => p_value < alpha,
            Direction::Backward => p_value > alpha,
        }
    }

    pub fn rank(self, a: (f64, f64), b: (f64, f64)) -> std::cmp::Ordering {
        match self {
            Direction::Forward => a.0.total_cmp(&b.0),
            Direction::Backward => b.0.total_cmp(&a.0),
        }
        .then(a.1.total_cmp(&b.1))
    }
}

/// Score one candidate against the round's reference.
pub fn lrt(reference_ofv: f64, candidate_ofv: f64, df: usize, direction: Direction) -> LrtResult {
    let delta_ofv = candidate_ofv - reference_ofv;
    let statistic = direction.statistic(delta_ofv);
    LrtResult {
        delta_ofv,
        statistic,
        df,
        p_value: chi2_sf(statistic, df),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// PsN 5.7.1's hard-coded (alpha, df) -> critical value table
    #[test]
    fn matches_psn_critical_values() {
        let cases: &[(f64, usize, f64)] = &[
            // (critical value, df, alpha)
            (3.841458820694124, 1, 0.05),
            (5.991464547107979, 2, 0.05),
            (7.814727903251179, 3, 0.05),
            (9.487729036781154, 4, 0.05),
            (10.827566170662733, 1, 0.001),
            (13.815510557964274, 2, 0.001),
            (16.26623619623813, 3, 0.001),
            (18.46682695290317, 4, 0.001),
        ];
        for &(x, df, alpha) in cases {
            let p = chi2_sf(x, df);
            assert!(
                (p - alpha).abs() < 1e-9,
                "chi2_sf({x}, {df}) = {p}, expected {alpha}"
            );
        }
    }

    /// The critical values PsN hard-codes, recovered from the alphas.
    #[test]
    fn inverse_recovers_the_critical_values() {
        for (x, df, alpha) in [
            (3.841458820694124, 1, 0.05),
            (5.991464547107979, 2, 0.05),
            (10.827566170662733, 1, 0.001),
            (18.46682695290317, 4, 0.001),
        ] {
            assert!((chi2_isf(alpha, df) - x).abs() < 1e-8, "{alpha} {df}");
        }
        assert!(chi2_isf(0.05, 0).is_nan());
        assert!(chi2_isf(0.0, 1).is_nan());
    }

    #[test]
    fn edge_cases() {
        assert_eq!(chi2_sf(0.0, 1), 1.0);
        assert_eq!(chi2_sf(-5.0, 1), 1.0);
        assert!(chi2_sf(1000.0, 1) < 1e-100);
        // large x path
        assert!((chi2_sf(20.0, 1) - 7.744216431e-6).abs() < 1e-12);
    }

    #[test]
    fn forward_lrt_orientation() {
        // Candidate improves by 10 points
        let r = lrt(1000.0, 990.0, 1, Direction::Forward);
        assert_eq!(r.delta_ofv, -10.0);
        assert_eq!(r.statistic, 10.0);
        assert!(r.p_value < 0.05);

        // Candidate is worse: not significant, p = 1
        let r = lrt(1000.0, 1005.0, 1, Direction::Forward);
        assert_eq!(r.delta_ofv, 5.0);
        assert_eq!(r.statistic, 0.0);
        assert_eq!(r.p_value, 1.0);
    }

    #[test]
    fn backward_lrt_orientation() {
        // Dropping the covariate raises OFV by 15 -> it is needed (significant)
        let r = lrt(1000.0, 1015.0, 1, Direction::Backward);
        assert_eq!(r.delta_ofv, 15.0);
        assert_eq!(r.statistic, 15.0);
        assert!(r.p_value < 0.001);

        // Dropping barely changes OFV -> droppable
        let r = lrt(1000.0, 1000.5, 1, Direction::Backward);
        assert!(r.p_value > 0.4);
    }

    #[test]
    fn ranking_and_alpha_follow_the_phase() {
        use std::cmp::Ordering::*;
        assert_eq!(Direction::Forward.rank((0.01, -5.0), (0.05, -9.0)), Less);
        assert_eq!(Direction::Forward.rank((0.01, -9.0), (0.01, -5.0)), Less);
        assert_eq!(Direction::Backward.rank((0.5, 1.0), (0.01, 9.0)), Less);
        assert!(Direction::Forward.meets(0.01, 0.05));
        assert!(!Direction::Forward.meets(0.05, 0.05));
        assert!(Direction::Backward.meets(0.05, 0.001));
    }
}
