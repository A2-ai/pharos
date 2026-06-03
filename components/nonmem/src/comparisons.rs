use anyhow::{Result as AnyhowResult, anyhow, bail};
use serde::{Deserialize, Serialize};
use statrs::distribution::{ChiSquared, ContinuousCDF};
use std::path::Path;

use crate::LineageTree;
use crate::metrics::InformationCriteria;
use crate::output_files::{get_summary, lst::extract_model, lst::find_lst};
use crate::run::metadata::{RUN_START_FILENAME, RunStartFile};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Lrt {
    Computed(LikelihoodRatioTest),
    NotNested,
    NoAddedParameters,
}

impl std::fmt::Display for Lrt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Lrt::Computed(_) => write!(f, "Computed"),
            Lrt::NotNested => write!(f, "Not Nested"),
            Lrt::NoAddedParameters => write!(f, "No Added Parameters"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LikelihoodRatioTest {
    pub df: usize,
    pub p_value: f64,
}

impl LikelihoodRatioTest {
    /// `statistic` is the LRT test statistic: reduced.ofv − full.ofv
    /// (≥ 0 when the full model fits better).
    pub fn new(statistic: f64, df: usize) -> AnyhowResult<Self> {
        let p_value = ChiSquared::new(df as f64)?.sf(statistic);
        Ok(Self { df, p_value })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ModelComparison {
    pub first_ic: InformationCriteria,
    pub second_ic: InformationCriteria,
    /// Deltas follow input order: `first − second`.
    pub delta_ofv: f64,
    pub delta_aic: f64,
    pub delta_bic: f64,
    pub lrt: Lrt,
}

impl ModelComparison {
    fn new(
        first_info: &InformationCriteria,
        second_info: &InformationCriteria,
        nested: bool,
    ) -> AnyhowResult<Self> {
        // Deltas follow input order.
        let delta_ofv = first_info.ofv - second_info.ofv;
        let delta_aic = first_info.aic - second_info.aic;
        let delta_bic = first_info.bic - second_info.bic;

        // The LRT orients by parameter count, independent of input order: the
        // model with more estimated parameters is the "full" one.
        let (full, reduced) =
            if first_info.n_estimated_parameters >= second_info.n_estimated_parameters {
                (first_info, second_info)
            } else {
                (second_info, first_info)
            };
        let df = full.n_estimated_parameters - reduced.n_estimated_parameters;

        let lrt = if !nested {
            Lrt::NotNested
        } else if df == 0 {
            Lrt::NoAddedParameters
        } else {
            Lrt::Computed(LikelihoodRatioTest::new(reduced.ofv - full.ofv, df)?)
        };

        Ok(Self {
            first_ic: *first_info,
            second_ic: *second_info,
            delta_ofv,
            delta_aic,
            delta_bic,
            lrt,
        })
    }

    /// Validates models meet requirements for comparison.
    /// 1. Same final estimation method
    /// 2. Same observations: identical dataset (file hash), row selection
    ///    (IGNORE/ACCEPT), and column mapping ($INPUT)
    /// 3. Same number of observations
    /// Computes whether the models are nested for LRT
    pub fn compare_runs<P: AsRef<Path>>(first_dir: P, second_dir: P) -> AnyhowResult<Self> {
        let first_dir = first_dir.as_ref();
        let second_dir = second_dir.as_ref();

        let first_model = extract_model(find_lst(first_dir)?)?;
        let second_model = extract_model(find_lst(second_dir)?)?;

        // Summaries contain InfoCriteria and Est methods for guards on comparison
        let first_summary = get_summary(first_dir, None, false)?;
        let second_summary = get_summary(second_dir, None, false)?;

        let first_final_est = first_summary
            .final_estimation_method()
            .ok_or_else(|| anyhow!("no estimation method found in {first_dir:?}"))?;
        let second_final_est = second_summary
            .final_estimation_method()
            .ok_or_else(|| anyhow!("no estimation method found in {second_dir:?}"))?;

        if first_final_est != second_final_est {
            bail!("final estimation methods differ: {first_final_est} vs {second_final_est}")
        };

        // The same observations must enter both objective functions for ΔOFV/LRT
        // to be valid: identical dataset content (file hash), identical row
        // selection (IGNORE/ACCEPT), and identical column mapping ($INPUT).
        let first_start = RunStartFile::load(first_dir.join(RUN_START_FILENAME))?;
        let second_start = RunStartFile::load(second_dir.join(RUN_START_FILENAME))?;
        if first_start.dataset_hashes.blake3 != second_start.dataset_hashes.blake3 {
            bail!("datasets differ (file hash mismatch); comparison not valid")
        }
        if !same_set(&first_model.data.ignore, &second_model.data.ignore)
            || !same_set(&first_model.data.accept, &second_model.data.accept)
        {
            bail!("IGNORE/ACCEPT filters differ; comparison not valid")
        }
        if !same_set(&first_model.input_columns, &second_model.input_columns) {
            bail!("$INPUT columns differ; comparison not valid")
        }

        // Nestedness from lineage. If we can't resolve it, fall back to
        // not-nested rather than failing the whole comparison.
        let nested = LineageTree::from_project()
            .and_then(|tree| tree.runs_related(first_dir, second_dir))
            .unwrap_or(false);

        let first_ic = first_summary
            .final_information_criteria()
            .ok_or_else(|| anyhow!("no information criteria for final method in {first_dir:?}"))?;
        let second_ic = second_summary
            .final_information_criteria()
            .ok_or_else(|| anyhow!("no information criteria for final method in {second_dir:?}"))?;

        if first_ic.n_observations != second_ic.n_observations {
            bail!("models have differing number of observations")
        }

        ModelComparison::new(&first_ic, &second_ic, nested)
    }
}

/// Order-insensitive equality for $DATA IGNORE/ACCEPT filters and $INPUT
/// Needs only `PartialEq` — `DataFilter` holds an `f64`, so it can't be `Ord`/`Hash`.
fn same_set<T: PartialEq>(a: &[T], b: &[T]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut matched = vec![false; b.len()];
    a.iter().all(|x| {
        match b.iter().enumerate().position(|(i, y)| !matched[i] && y == x) {
            Some(i) => {
                matched[i] = true;
                true
            }
            None => false,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::*;

    #[test]
    fn test_model_comparison() {
        let base = InformationCriteria::new(1000.0, 6, 320);
        let full = InformationCriteria::new(981.326, 7, 320);
        let alt = InformationCriteria::new(997.5000, 7, 320);

        let comp = ModelComparison::new(&full, &base, true).unwrap();
        assert!((comp.delta_ofv - -18.674).abs() < 1e-10);
        let Lrt::Computed(lrt) = comp.lrt else {
            panic!("expected a computed LRT")
        };
        assert!(lrt.p_value < 0.05);

        let comp = ModelComparison::new(&alt, &base, true).unwrap();
        assert!((comp.delta_ofv - -2.5).abs() < 1e-10);
        let Lrt::Computed(lrt) = comp.lrt else {
            panic!("expected a computed LRT")
        };
        assert!(lrt.p_value > 0.05);

        let comp = ModelComparison::new(&alt, &base, false).unwrap();
        assert!((comp.delta_ofv - -2.5).abs() < 1e-10);
        assert_eq!(comp.lrt, Lrt::NotNested);
    }
}
