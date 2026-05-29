use crate::metrics::InformationCriteria;
use anyhow::Result as AnyhowResult;
use serde::{Deserialize, Serialize};
use statrs::distribution::{ChiSquared, ContinuousCDF};

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
    pub delta_ofv: f64,
    pub delta_aic: f64,
    pub delta_bic: f64,
    pub lrt: Option<LikelihoodRatioTest>,
}

impl ModelComparison {
    pub fn new(
        reduced_info: InformationCriteria,
        full_info: InformationCriteria,
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
            Some(df) => {
                if nested && df > 0 && same_n_obs {
                    Some(LikelihoodRatioTest::new(delta_ofv, df)?)
                } else {
                    None
                }
            }
            _ => None,
        };

        Ok(Self {
            delta_ofv,
            delta_aic,
            delta_bic,
            lrt,
        })
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

        let comp = ModelComparison::new(base, full, true).unwrap();
        assert!((comp.delta_ofv - -18.674).abs() < 1e-10);
        let lrt = comp.lrt.unwrap();
        assert!(lrt.p_value < 0.05);

        let comp = ModelComparison::new(base, alt, true).unwrap();
        assert!((comp.delta_ofv - -2.5).abs() < 1e-10);
        let lrt = comp.lrt.unwrap();
        assert!(lrt.p_value > 0.05);

        let comp = ModelComparison::new(base, diff, true).unwrap();
        assert!((comp.delta_ofv - -500.0).abs() < 1e-10);
        assert!(comp.lrt.is_none());
    }
}
