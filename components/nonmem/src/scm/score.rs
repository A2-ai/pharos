use serde::{Deserialize, Serialize};

use super::Direction;

/// ln Γ(x) via the Lanczos approximation (g = 7, n = 9), accurate to ~1e-13
/// for x > 0.
fn ln_gamma(x: f64) -> f64 {
    const COEFFS: [f64; 8] = [
        676.5203681218851,
        -1259.1392167224028,
        771.323_428_777_653_1,
        -176.615_029_162_140_6,
        12.507343278686905,
        -0.13857109526572012,
        9.984_369_578_019_572e-6,
        1.5056327351493116e-7,
    ];

    if x < 0.5 {
        // Reflection formula
        let pi = std::f64::consts::PI;
        return (pi / (pi * x).sin()).ln() - ln_gamma(1.0 - x);
    }

    let x = x - 1.0;
    let mut a = 0.999_999_999_999_809_9;
    let t = x + 7.5;
    for (i, &c) in COEFFS.iter().enumerate() {
        a += c / (x + (i as f64) + 1.0);
    }
    0.5 * (2.0 * std::f64::consts::PI).ln() + (x + 0.5) * t.ln() - t + a.ln()
}

/// Regularized lower incomplete gamma P(a, x) by series expansion (x < a + 1).
fn gamma_p_series(a: f64, x: f64) -> f64 {
    let mut sum = 1.0 / a;
    let mut term = sum;
    let mut n = a;
    for _ in 0..500 {
        n += 1.0;
        term *= x / n;
        sum += term;
        if term.abs() < sum.abs() * 1e-16 {
            break;
        }
    }
    sum * (-x + a * x.ln() - ln_gamma(a)).exp()
}

/// Regularized upper incomplete gamma Q(a, x) by continued fraction (x >= a + 1).
fn gamma_q_cf(a: f64, x: f64) -> f64 {
    const TINY: f64 = 1e-300;
    let mut b = x + 1.0 - a;
    let mut c = 1.0 / TINY;
    let mut d = 1.0 / b;
    let mut h = d;
    for i in 1..500 {
        let an = -(i as f64) * ((i as f64) - a);
        b += 2.0;
        d = an * d + b;
        if d.abs() < TINY {
            d = TINY;
        }
        c = b + an / c;
        if c.abs() < TINY {
            c = TINY;
        }
        d = 1.0 / d;
        let delta = d * c;
        h *= delta;
        if (delta - 1.0).abs() < 1e-16 {
            break;
        }
    }
    (-x + a * x.ln() - ln_gamma(a)).exp() * h
}

/// Chi-square survival function: P(X > x) for X ~ chi2(df).
pub fn chi2_sf(x: f64, df: usize) -> f64 {
    if x <= 0.0 || df == 0 {
        return 1.0;
    }
    let a = df as f64 / 2.0;
    let half_x = x / 2.0;
    if half_x < a + 1.0 {
        1.0 - gamma_p_series(a, half_x)
    } else {
        gamma_q_cf(a, half_x)
    }
}

/// Result of one candidate-vs-reference likelihood-ratio test.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LrtResult {
    /// candidate OFV − reference OFV; negative when the candidate improves on
    /// the reference (the sign convention requested by the science team).
    pub delta_ofv: f64,
    /// The tested statistic (never negative; a "wrong-way" delta clamps to 0).
    pub statistic: f64,
    pub df: usize,
    pub p_value: f64,
}

/// Score one candidate against the round's reference.
///
/// Forward: candidate = reference + one released covariate (candidate is the
/// full model); the statistic is the OFV drop the covariate buys.
/// Backward: candidate = reference − one covariate (candidate is the reduced
/// model); the statistic is the OFV rise removing the covariate costs.
pub fn lrt(reference_ofv: f64, candidate_ofv: f64, df: usize, direction: Direction) -> LrtResult {
    let delta_ofv = candidate_ofv - reference_ofv;
    let statistic = match direction {
        Direction::Forward => (-delta_ofv).max(0.0),
        Direction::Backward => delta_ofv.max(0.0),
    };
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

    /// PsN 5.7.1's hard-coded (alpha, df) -> critical value table.
    #[test]
    fn matches_psn_critical_values() {
        let cases: &[(f64, usize, f64)] = &[
            // (critical value, df, alpha)
            (3.841458820694124, 1, 0.05),
            (5.991464547107979, 2, 0.05),
            (7.814727903251179, 3, 0.05),
            (9.487729036781154, 4, 0.05),
            (6.634896601021213, 1, 0.01),
            (9.21034037197618, 2, 0.01),
            (11.344866730144373, 3, 0.01),
            (13.276704135987622, 4, 0.01),
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

    #[test]
    fn edge_cases() {
        assert_eq!(chi2_sf(0.0, 1), 1.0);
        assert_eq!(chi2_sf(-5.0, 1), 1.0);
        assert!(chi2_sf(1000.0, 1) < 1e-100);
        // large x path (continued fraction)
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
}
