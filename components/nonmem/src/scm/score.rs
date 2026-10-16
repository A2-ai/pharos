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

/// Score one candidate against the round's reference: `(delta OFV, p-value)`.
pub fn lrt(reference_ofv: f64, candidate_ofv: f64, df: usize, direction: Direction) -> (f64, f64) {
    let delta_ofv = candidate_ofv - reference_ofv;
    (delta_ofv, chi2_sf(direction.statistic(delta_ofv), df))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// PsN 5.7.1's hard-coded (alpha, df) -> critical value table, recovered
    /// in both directions.
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
            assert!((chi2_isf(alpha, df) - x).abs() < 1e-8, "{alpha} {df}");
        }
        assert_eq!(chi2_sf(0.0, 1), 1.0);
        assert_eq!(chi2_sf(-5.0, 1), 1.0);
        assert!(chi2_isf(0.05, 0).is_nan());
        assert!(chi2_isf(0.0, 1).is_nan());
    }

    #[test]
    fn lrt_ranking_and_alpha_follow_the_phase() {
        use std::cmp::Ordering::*;

        // forward: a 10-point improvement is significant, a worse fit is p = 1
        let (delta, p) = lrt(1000.0, 990.0, 1, Direction::Forward);
        assert_eq!((delta, Direction::Forward.statistic(delta)), (-10.0, 10.0));
        assert!(p < 0.05);
        assert_eq!(lrt(1000.0, 1005.0, 1, Direction::Forward), (5.0, 1.0));

        // backward: dropping a needed covariate raises OFV by 15; a
        // half-point rise is droppable
        let (delta, p) = lrt(1000.0, 1015.0, 1, Direction::Backward);
        assert_eq!(Direction::Backward.statistic(delta), 15.0);
        assert!(p < 0.001);
        assert!(lrt(1000.0, 1000.5, 1, Direction::Backward).1 > 0.4);

        assert_eq!(Direction::Forward.rank((0.01, -5.0), (0.05, -9.0)), Less);
        assert_eq!(Direction::Forward.rank((0.01, -9.0), (0.01, -5.0)), Less);
        assert_eq!(Direction::Backward.rank((0.5, 1.0), (0.01, 9.0)), Less);
        assert!(Direction::Forward.meets(0.01, 0.05));
        assert!(!Direction::Forward.meets(0.05, 0.05));
        assert!(Direction::Backward.meets(0.05, 0.001));
    }
}
