use anyhow::{Result as AnyhowResult, anyhow, bail};
use serde::{Deserialize, Serialize};
use statrs::distribution::{ChiSquared, ContinuousCDF};
use std::path::Path;

use crate::LineageTree;
use crate::metrics::InformationCriteria;
use crate::output_files::get_summary;
use crate::run::metadata::{RUN_START_FILENAME, RunStartFile};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LikelihoodRatioTest {
    pub df: usize,
    pub p_value: f64,
}

impl LikelihoodRatioTest {
    pub fn new(delta_ofv: f64, df: usize) -> AnyhowResult<Self> {
        // test-statistic for LRT is reduced - full (so neagtive delta ofv)
        let p_value = ChiSquared::new(df as f64)?.sf(-delta_ofv);
        Ok(Self { df, p_value })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ModelComparison {
    pub full_ic: InformationCriteria,
    pub reduced_ic: InformationCriteria,
    pub delta_ofv: f64,
    pub delta_aic: f64,
    pub delta_bic: f64,
    pub lrt: Option<LikelihoodRatioTest>,
}

impl ModelComparison {
    fn new(
        reduced_info: &InformationCriteria,
        full_info: &InformationCriteria,
        nested: bool,
    ) -> AnyhowResult<Self> {
        let delta_ofv = full_info.ofv - reduced_info.ofv;
        let delta_aic = full_info.aic - reduced_info.aic;
        let delta_bic = full_info.bic - reduced_info.bic;

        let df = full_info
            .n_estimated_parameters
            .checked_sub(reduced_info.n_estimated_parameters);

        let same_n_obs = reduced_info.n_observations == full_info.n_observations;

        // LRT is only valid for nested models with >= 1 additional parameter fitted
        // and same number of observations
        let lrt = match df {
            Some(df) if nested && df > 0 && same_n_obs => {
                Some(LikelihoodRatioTest::new(delta_ofv, df)?)
            }
            _ => None,
        };

        Ok(Self {
            full_ic: *full_info,
            reduced_ic: *reduced_info,
            delta_ofv,
            delta_aic,
            delta_bic,
            lrt,
        })
    }

    pub fn compare_runs<P: AsRef<Path>>(full_dir: P, reduced_dir: P) -> AnyhowResult<Self> {
        let full_dir = full_dir.as_ref();
        let reduced_dir = reduced_dir.as_ref();

        // Summaries contain InfoCriteria and Est methods for guards on comparison
        let full_summary = get_summary(full_dir, None, false)?;
        let reduced_summary = get_summary(reduced_dir, None, false)?;

        let full_final_est = full_summary
            .lst
            .run_details
            .estimation_methods
            .last()
            .ok_or_else(|| anyhow!("no estimation method found in {full_dir:?}"))?;
        let reduced_final_est = reduced_summary
            .lst
            .run_details
            .estimation_methods
            .last()
            .ok_or_else(|| anyhow!("no estimation method found in {reduced_dir:?}"))?;

        if full_final_est != reduced_final_est {
            bail!(
                "Full ({full_final_est}) and reduced ({reduced_final_est}) final estimation methods do not match"
            )
        };

        // Nestedness from lineage. Recovering each model's source path depends on
        // pharos_start.json (model_canonical_path); if any step fails (no start
        // file, not in a project, no metadata) we can't confirm nesting, so fall
        // back to not-nested rather than failing the whole comparison.
        let nested = || -> AnyhowResult<bool> {
            let full_model_path =
                RunStartFile::load(full_dir.join(RUN_START_FILENAME))?.model_canonical_path;
            let reduced_model_path =
                RunStartFile::load(reduced_dir.join(RUN_START_FILENAME))?.model_canonical_path;
            LineageTree::from_project()?.is_related(&full_model_path, &reduced_model_path)
        }()
        .unwrap_or(false);

        let reduced_ic = reduced_summary
            .information_criteria
            .last()
            .copied()
            .flatten()
            .ok_or_else(|| {
                anyhow!("no information criteria for final method in {reduced_dir:?}")
            })?;
        let full_ic = full_summary
            .information_criteria
            .last()
            .copied()
            .flatten()
            .ok_or_else(|| anyhow!("no information criteria for final method in {full_dir:?}"))?;

        ModelComparison::new(&reduced_ic, &full_ic, nested)
    }
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
        let diff = InformationCriteria::new(500.0, 8, 120);

        let comp = ModelComparison::new(&base, &full, true).unwrap();
        assert!((comp.delta_ofv - -18.674).abs() < 1e-10);
        let lrt = comp.lrt.unwrap();
        assert!(lrt.p_value < 0.05);

        let comp = ModelComparison::new(&base, &alt, true).unwrap();
        assert!((comp.delta_ofv - -2.5).abs() < 1e-10);
        let lrt = comp.lrt.unwrap();
        assert!(lrt.p_value > 0.05);

        let comp = ModelComparison::new(&base, &diff, true).unwrap();
        assert!((comp.delta_ofv - -500.0).abs() < 1e-10);
        assert!(comp.lrt.is_none());

        let comp = ModelComparison::new(&base, &alt, false).unwrap();
        assert!((comp.delta_ofv - -2.5).abs() < 1e-10);
        assert!(comp.lrt.is_none());
    }
}
