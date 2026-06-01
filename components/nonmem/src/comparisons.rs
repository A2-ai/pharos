use anyhow::{Result as AnyhowResult, anyhow, bail};
use serde::{Deserialize, Serialize};
use statrs::distribution::{ChiSquared, ContinuousCDF};
use std::path::Path;

use crate::LineageTree;
use crate::metrics::InformationCriteria;
use crate::output_files::get_summary;

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
    pub lrt: Lrt,
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

        // LRT is only valid for nested models with >= 1 additional parameter fitted
        // and same number of observations
        let lrt = match df {
            Some(df) => {
                if !nested {
                    Lrt::NotNested
                } else if df == 0 {
                    Lrt::NoAddedParameters
                } else {
                    Lrt::Computed(LikelihoodRatioTest::new(delta_ofv, df)?)
                }
            }
            None => Lrt::NoAddedParameters,
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

    /// Validates models meet requirements for comparison.
    /// 1. Same final estimation method
    /// 2. Same number of observations
    /// Computes whether the models are nested for LRT
    pub fn compare_runs<P: AsRef<Path>>(full_dir: P, reduced_dir: P) -> AnyhowResult<Self> {
        let full_dir = full_dir.as_ref();
        let reduced_dir = reduced_dir.as_ref();

        // Summaries contain InfoCriteria and Est methods for guards on comparison
        let full_summary = get_summary(full_dir, None, false)?;
        let reduced_summary = get_summary(reduced_dir, None, false)?;

        let full_final_est = full_summary
            .final_estimation_method()
            .ok_or_else(|| anyhow!("no estimation method found in {full_dir:?}"))?;
        let reduced_final_est = reduced_summary
            .final_estimation_method()
            .ok_or_else(|| anyhow!("no estimation method found in {reduced_dir:?}"))?;

        if full_final_est != reduced_final_est {
            bail!(
                "Full ({full_final_est}) and reduced ({reduced_final_est}) final estimation methods do not match"
            )
        };

        // Nestedness from lineage. If we can't resolve it so fall back to
        // not-nested rather than failing the whole comparison.
        let nested = LineageTree::from_project()
            .and_then(|tree| tree.runs_related(full_dir, reduced_dir))
            .unwrap_or(false);

        let reduced_ic = reduced_summary
            .final_information_criteria()
            .ok_or_else(|| {
                anyhow!("no information criteria for final method in {reduced_dir:?}")
            })?;

        let full_ic = full_summary
            .final_information_criteria()
            .ok_or_else(|| anyhow!("no information criteria for final method in {full_dir:?}"))?;

        if reduced_ic.n_observations != full_ic.n_observations {
            bail!("Models have differeing number of observations")
        }

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

        let comp = ModelComparison::new(&base, &full, true).unwrap();
        assert!((comp.delta_ofv - -18.674).abs() < 1e-10);
        let Lrt::Computed(lrt) = comp.lrt else {
            panic!("expected a computed LRT")
        };
        assert!(lrt.p_value < 0.05);

        let comp = ModelComparison::new(&base, &alt, true).unwrap();
        assert!((comp.delta_ofv - -2.5).abs() < 1e-10);
        let Lrt::Computed(lrt) = comp.lrt else {
            panic!("expected a computed LRT")
        };
        assert!(lrt.p_value > 0.05);

        let comp = ModelComparison::new(&base, &alt, false).unwrap();
        assert!((comp.delta_ofv - -2.5).abs() < 1e-10);
        assert_eq!(comp.lrt, Lrt::NotNested);
    }
}
