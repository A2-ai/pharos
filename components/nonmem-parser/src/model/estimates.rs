use std::collections::HashMap;

use anyhow::{Result as AnyhowResult, bail};
use rand::Rng;
use rand::SeedableRng;
use rand::rngs::StdRng;

use crate::ast::{BlockStructure, OmegaSigmaBlock};
use crate::lexer::SpannedToken;
use crate::model::Model;

/// Count the number of decimal places in a numeric string representation.
fn num_decimal_places(s: &str) -> usize {
    if let Some(dot_pos) = s.find('.') {
        s.len() - dot_pos - 1
    } else {
        0
    }
}

fn round_arbitrary_precision(original: &str, new_value: f64) -> f64 {
    if new_value == 0.0 {
        return new_value;
    }
    let num_decimals_wanted = num_decimal_places(original) + 2;
    let factor = 10_f64.powi(num_decimals_wanted as i32);
    (new_value * factor).round() / factor
}

/// Apply jittering to a parameter value using uniform distribution with boundary enforcement.
fn apply_jittering(
    value: f64,
    jitter_percentage: f64,
    rng: &mut StdRng,
    lower_bound: Option<f64>,
    upper_bound: Option<f64>,
    original_str: &str,
) -> f64 {
    let random_factor: f64 = rng.random_range(-1.0..=1.0);
    let perturbation_multiplier = 1.0 + (jitter_percentage * random_factor);
    let mut jittered_value = value * perturbation_multiplier;

    // Round first so that subsequent bounds checks are against the final precision
    jittered_value = round_arbitrary_precision(original_str, jittered_value);

    // Smallest number we can add matching the required precision
    let precision_epsilon = 10_f64.powi(-((num_decimal_places(original_str) + 2) as i32));

    // Enforce strict lower bound: jittered must be > lower
    if let Some(lower) = lower_bound
        && jittered_value <= lower
    {
        jittered_value = lower + precision_epsilon;
    }

    // Enforce strict upper bound: jittered must be < upper
    if let Some(upper) = upper_bound
        && jittered_value >= upper
    {
        jittered_value = upper - precision_epsilon;
    }

    jittered_value
}

/// Walk omega/sigma blocks and update estimates from a HashMap keyed by
/// coordinate names like `OMEGA(1,1)` or `SIGMA(2,1)`.
fn update_block_estimates(
    blocks: &mut [OmegaSigmaBlock],
    tokens: &mut [SpannedToken],
    estimates: &HashMap<String, f64>,
    prefix: &str,
) {
    let mut param_counter: usize = 1;

    for block in blocks.iter_mut() {
        match &block.structure {
            BlockStructure::Diagonal => {
                for param in block.parameters.iter_mut() {
                    if !block.fixed && !param.fixed {
                        let name = format!("{prefix}({param_counter},{param_counter})");
                        if let Some(&estimate) = estimates.get(&name) {
                            let original_str = &tokens[param.value_idx].text;
                            let rounded = round_arbitrary_precision(original_str, estimate);
                            param.value = rounded;
                            tokens[param.value_idx].text = rounded.to_string();
                        }
                    }
                    param_counter += 1;
                }
            }
            BlockStructure::Block { size } => {
                let base = param_counter;
                let mut param_idx = 0;
                for row in 0..*size {
                    for col in 0..=row {
                        if param_idx < block.parameters.len() {
                            let param = &mut block.parameters[param_idx];
                            if !block.fixed && !param.fixed {
                                let name = format!("{prefix}({},{})", base + row, base + col);
                                if let Some(&estimate) = estimates.get(&name) {
                                    let original_str = &tokens[param.value_idx].text;
                                    let rounded = round_arbitrary_precision(original_str, estimate);
                                    param.value = rounded;
                                    tokens[param.value_idx].text = rounded.to_string();
                                }
                            }
                            param_idx += 1;
                        }
                    }
                }
                param_counter += size;
            }
            BlockStructure::BlockSame { size, repeats } => {
                param_counter += size * repeats;
            }
        }
    }
}

impl Model {
    /// Update initial parameter estimates from a map keyed by `THETA1`, `OMEGA(1,1)`, etc.
    /// Optionally jitters theta values (only). Omega/sigma are never jittered.
    pub fn update_initial_estimates(
        &mut self,
        estimates: &HashMap<String, f64>,
        jitter: Option<f64>,
        seed: Option<u64>,
        excluded: &[String],
    ) {
        let mut rng = jitter.map(|_| {
            if let Some(seed) = seed {
                StdRng::seed_from_u64(seed)
            } else {
                StdRng::from_os_rng()
            }
        });

        // Update thetas + optional jitter
        for (i, param) in self.thetas.iter_mut().enumerate() {
            if param.fixed {
                continue;
            }
            let name = format!("THETA{}", i + 1);
            if let Some(&estimate) = estimates.get(&name) {
                let original_str = &self.tokens[param.init_idx].text;
                let mut final_value = estimate;

                if let (Some(jitter_pct), Some(rng)) = (jitter, rng.as_mut())
                    && !excluded.contains(&name)
                {
                    final_value = apply_jittering(
                        estimate,
                        jitter_pct,
                        rng,
                        param.lower,
                        param.upper,
                        original_str,
                    );
                }

                let rounded = round_arbitrary_precision(original_str, final_value);
                param.init = rounded;
                self.tokens[param.init_idx].text = rounded.to_string();
            }
        }

        // Update omegas (no jitter)
        update_block_estimates(&mut self.omega_blocks, &mut self.tokens, estimates, "OMEGA");

        // Update sigmas (no jitter)
        update_block_estimates(&mut self.sigma_blocks, &mut self.tokens, estimates, "SIGMA");
    }

    /// Create `num_retries` copies of this model, each with theta parameters
    /// jittered by `degree` (0..1). Only thetas are perturbed; omega/sigma are unchanged.
    pub fn theta_perturbation(
        &self,
        degree: f64,
        num_retries: usize,
        seed: Option<u64>,
    ) -> AnyhowResult<Vec<Model>> {
        if degree <= 0.0 || degree >= 1.0 {
            bail!("Degree must be between 0 and 1 (exclusive)");
        }

        let mut rng = if let Some(seed) = seed {
            StdRng::seed_from_u64(seed)
        } else {
            StdRng::from_os_rng()
        };

        let mut models = Vec::with_capacity(num_retries);

        for _ in 0..num_retries {
            let mut new_model = self.clone();
            for param in new_model.thetas.iter_mut() {
                if param.fixed {
                    continue;
                }
                let original_str = new_model.tokens[param.init_idx].text.clone();
                let jittered = apply_jittering(
                    param.init,
                    degree,
                    &mut rng,
                    param.lower,
                    param.upper,
                    &original_str,
                );
                param.init = jittered;
                new_model.tokens[param.init_idx].text = jittered.to_string();
            }
            models.push(new_model);
        }

        Ok(models)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_model(input: &str) -> Model {
        Model::parse(input).unwrap()
    }

    #[test]
    fn round_to_significant_digits() {
        let tests = vec![
            (0.0, 0.0, 0.0),
            (1.5, 1.657474576, 1.657),
            (-0.5, -0.44563545364634, -0.446),
            (1.0, 1.23456789, 1.23),
            (3.14, 3.12828123182, 3.1283),
        ];

        for (n, n2, expected) in tests {
            assert_eq!(round_arbitrary_precision(&n.to_string(), n2), expected);
        }
    }

    #[test]
    fn jittered_value_strictly_between_bounds() {
        let cases: &[(f64, f64, Option<f64>, Option<f64>, &str)] = &[
            (-0.991385, 0.2, Some(-1.0), Some(1.0), "-9.91385E-01"),
            (0.01, 0.99, Some(0.0), None, "0.01"),
            (0.99, 0.2, None, Some(1.0), "0.99"),
            (0.999, 0.1, None, Some(1.0), "0.999"),
            (0.0, 0.2, Some(0.0), Some(10.0), "0.0"),
        ];

        for (value, jitter, lower, upper, original) in cases {
            for seed in 0..500 {
                let mut rng = StdRng::seed_from_u64(seed);
                let result = apply_jittering(*value, *jitter, &mut rng, *lower, *upper, original);
                if let Some(lo) = lower {
                    assert!(
                        result > *lo,
                        "seed {seed}, case {original}: {result} <= lower {lo}"
                    );
                }
                if let Some(hi) = upper {
                    assert!(
                        result < *hi,
                        "seed {seed}, case {original}: {result} >= upper {hi}"
                    );
                }
            }
        }
    }

    #[test]
    fn update_initial_estimates_basic() {
        let input = "\
$PROBLEM test
$INPUT ID
$DATA data.csv
$THETA (0, 1.5, 10)
$THETA 0.5 FIX
$THETA (-1, 0.3)
";
        let mut model = parse_model(input);

        let estimates: HashMap<String, f64> = [
            ("THETA1".into(), 2.0),
            ("THETA2".into(), 99.0),
            ("THETA3".into(), -0.1),
        ]
        .into_iter()
        .collect();

        model.update_initial_estimates(&estimates, None, None, &[]);

        // THETA1 updated (not fixed)
        assert!((model.thetas[0].init - 2.0).abs() < 1e-6);
        // THETA2 is fixed — should remain 0.5
        assert!((model.thetas[1].init - 0.5).abs() < 1e-6);
        // THETA3 updated
        assert!((model.thetas[2].init - (-0.1)).abs() < 1e-6);

        // Token text should reflect new values
        assert_eq!(model.tokens[model.thetas[0].init_idx].text, "2");
        assert_eq!(model.tokens[model.thetas[2].init_idx].text, "-0.1");
        // Fixed theta token should not have changed
        assert_eq!(model.tokens[model.thetas[1].init_idx].text, "0.5");
    }

    #[test]
    fn update_initial_estimates_omega_diagonal() {
        let input = "\
$PROBLEM test
$INPUT ID
$DATA data.csv
$THETA 1
$OMEGA 0.04
$OMEGA 0.09
";
        let mut model = parse_model(input);

        let estimates: HashMap<String, f64> =
            [("OMEGA(1,1)".into(), 0.05), ("OMEGA(2,2)".into(), 0.12)]
                .into_iter()
                .collect();

        model.update_initial_estimates(&estimates, None, None, &[]);

        assert!((model.omega_blocks[0].parameters[0].value - 0.05).abs() < 1e-6);
        assert!((model.omega_blocks[1].parameters[0].value - 0.12).abs() < 1e-6);

        let content = model.model_content();
        assert!(content.contains("0.05"), "content: {content}");
        assert!(content.contains("0.12"), "content: {content}");
    }

    #[test]
    fn update_initial_estimates_omega_block() {
        let input = "\
$PROBLEM test
$INPUT ID
$DATA data.csv
$THETA 1
$OMEGA BLOCK(2)
0.04
0.01 0.09
";
        let mut model = parse_model(input);

        let estimates: HashMap<String, f64> = [
            ("OMEGA(1,1)".into(), 0.05),
            ("OMEGA(2,1)".into(), 0.02),
            ("OMEGA(2,2)".into(), 0.10),
        ]
        .into_iter()
        .collect();

        model.update_initial_estimates(&estimates, None, None, &[]);

        assert!((model.omega_blocks[0].parameters[0].value - 0.05).abs() < 1e-6);
        assert!((model.omega_blocks[0].parameters[1].value - 0.02).abs() < 1e-6);
        assert!((model.omega_blocks[0].parameters[2].value - 0.10).abs() < 1e-6);
    }

    #[test]
    fn update_initial_estimates_with_jitter() {
        let input = "\
$PROBLEM test
$INPUT ID
$DATA data.csv
$THETA (0, 1.5, 10)
$THETA 0.5 FIX
$OMEGA 0.04
";
        let mut model = parse_model(input);

        let estimates: HashMap<String, f64> = [("THETA1".into(), 1.5), ("OMEGA(1,1)".into(), 0.04)]
            .into_iter()
            .collect();

        model.update_initial_estimates(&estimates, Some(0.2), Some(42), &[]);

        // Non-fixed theta should have been jittered
        assert!(
            (model.thetas[0].init - 1.5).abs() > 1e-10,
            "theta should have been jittered"
        );
        // Fixed theta should be unchanged
        assert!((model.thetas[1].init - 0.5).abs() < 1e-10);
        // Omega should be updated but NOT jittered
        assert!((model.omega_blocks[0].parameters[0].value - 0.04).abs() < 1e-6);
    }

    #[test]
    fn update_initial_estimates_jitter_respects_exclusions() {
        let input = "\
$PROBLEM test
$INPUT ID
$DATA data.csv
$THETA (0, 1.5, 10)
$THETA (0, 2.0, 10)
";
        let mut model = parse_model(input);

        let estimates: HashMap<String, f64> = [("THETA1".into(), 1.5), ("THETA2".into(), 2.0)]
            .into_iter()
            .collect();

        model.update_initial_estimates(&estimates, Some(0.2), Some(42), &["THETA1".to_string()]);

        // Excluded theta should be updated but not jittered
        assert!((model.thetas[0].init - 1.5).abs() < 1e-6);
        // Non-excluded theta should have been jittered
        assert!((model.thetas[1].init - 2.0).abs() > 1e-10);
    }

    #[test]
    fn theta_perturbation_creates_copies() {
        let input = "\
$PROBLEM test
$INPUT ID
$DATA data.csv
$THETA (0, 1.5, 10)
$THETA 0.5 FIX
";
        let model = parse_model(input);
        let models = model.theta_perturbation(0.2, 3, Some(123)).unwrap();

        assert_eq!(models.len(), 3);

        for m in &models {
            // Fixed theta should be unchanged in all copies
            assert!((m.thetas[1].init - 0.5).abs() < 1e-10);
            // Non-fixed theta should be within bounds
            assert!(m.thetas[0].init > 0.0);
            assert!(m.thetas[0].init < 10.0);
        }

        // Different copies should (very likely) have different values
        assert!(
            (models[0].thetas[0].init - models[1].thetas[0].init).abs() > 1e-10
                || (models[1].thetas[0].init - models[2].thetas[0].init).abs() > 1e-10
        );
    }

    #[test]
    fn theta_perturbation_deterministic_with_seed() {
        let input = "\
$PROBLEM test
$INPUT ID
$DATA data.csv
$THETA (0, 1.5, 10)
$THETA (-5, 0.3, 5)
";
        let model = parse_model(input);
        let models_a = model.theta_perturbation(0.2, 3, Some(42)).unwrap();
        let models_b = model.theta_perturbation(0.2, 3, Some(42)).unwrap();

        for (a, b) in models_a.iter().zip(models_b.iter()) {
            assert!((a.thetas[0].init - b.thetas[0].init).abs() < 1e-15);
            assert!((a.thetas[1].init - b.thetas[1].init).abs() < 1e-15);
        }
    }

    #[test]
    fn update_and_jitter_round_trip_model_content() {
        let input = "\
$PROBLEM test
$INPUT ID
$DATA data.csv
$THETA (0, 1.5, 10)
$OMEGA 0.04
$EST METHOD=0
";
        let mut model = parse_model(input);

        let estimates: HashMap<String, f64> =
            [("THETA1".into(), 2.345), ("OMEGA(1,1)".into(), 0.067)]
                .into_iter()
                .collect();

        model.update_initial_estimates(&estimates, None, None, &[]);

        // The model content should be valid and parseable
        let content = model.model_content();
        let reparsed = Model::parse(&content).unwrap();
        assert!((reparsed.thetas[0].init - model.thetas[0].init).abs() < 1e-10);
    }
}
