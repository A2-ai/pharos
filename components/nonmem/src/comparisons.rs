use anyhow::{Result as AnyhowResult, anyhow, bail};
use serde::{Deserialize, Serialize};
use statrs::distribution::{ChiSquared, ContinuousCDF};
use std::num::NonZeroUsize;
use std::path::Path;

use crate::LineageTree;
use crate::model_resolution::ModelLayout;
use crate::output_files::metrics::InformationCriteria;
use crate::output_files::{get_summary, lst::extract_model};
use crate::run::metadata::{RUN_START_FILENAME, RunStartFile};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Lrt {
    Computed(LikelihoodRatioTest),
    NotNested,
    NoAddedParameters,
    LineageUnavailable,
}

impl std::fmt::Display for Lrt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Lrt::Computed(_) => write!(f, "Computed"),
            Lrt::NotNested => write!(f, "Not Nested"),
            Lrt::NoAddedParameters => write!(f, "No Added Parameters"),
            Lrt::LineageUnavailable => write!(f, "No Lineage Metadata"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LikelihoodRatioTest {
    pub df: NonZeroUsize,
    pub p_value: f64,
}

impl LikelihoodRatioTest {
    /// `statistic` is the LRT test statistic: reduced.ofv − full.ofv
    /// (≥ 0 when the full model fits better).
    pub fn new(statistic: f64, df: NonZeroUsize) -> Self {
        let p_value = ChiSquared::new(df.get() as f64)
            .expect("df is non-zero usize")
            .sf(statistic);
        Self { df, p_value }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ModelComparison {
    pub candidate_ic: InformationCriteria,
    pub reference_ic: InformationCriteria,
    /// Deltas are `candidate − reference`, so a candidate that fits better than
    /// its reference reports a negative dOFV.
    pub delta_ofv: f64,
    pub delta_aic: f64,
    pub delta_bic: f64,
    pub lrt: Lrt,
}

impl ModelComparison {
    fn new(
        candidate_info: &InformationCriteria,
        reference_info: &InformationCriteria,
        nested: Option<bool>,
    ) -> Self {
        let delta_ofv = candidate_info.ofv - reference_info.ofv;
        let delta_aic = candidate_info.aic - reference_info.aic;
        let delta_bic = candidate_info.bic - reference_info.bic;

        // The LRT orients by parameter count, independent of the candidate and
        // reference roles: the model with more estimated parameters is the
        // "full" one.
        let (full, reduced) =
            if candidate_info.n_estimated_parameters >= reference_info.n_estimated_parameters {
                (candidate_info, reference_info)
            } else {
                (reference_info, candidate_info)
            };
        let df = full.n_estimated_parameters - reduced.n_estimated_parameters;

        let lrt = match nested {
            None => Lrt::LineageUnavailable,
            Some(false) => Lrt::NotNested,
            Some(true) => match NonZeroUsize::new(df) {
                None => Lrt::NoAddedParameters,
                Some(df) => Lrt::Computed(LikelihoodRatioTest::new(reduced.ofv - full.ofv, df)),
            },
        };

        Self {
            candidate_ic: *candidate_info,
            reference_ic: *reference_info,
            delta_ofv,
            delta_aic,
            delta_bic,
            lrt,
        }
    }

    /// Guards on estimation method and observations, because dOFV and the LRT
    /// are only meaningful when the same data entered both objective functions.
    pub fn compare_runs<P: AsRef<Path>>(
        candidate_dir: P,
        reference_dir: P,
        tree: &LineageTree,
    ) -> AnyhowResult<Self> {
        let candidate_dir = candidate_dir.as_ref();
        let reference_dir = reference_dir.as_ref();

        let candidate_start = RunStartFile::load(candidate_dir.join(RUN_START_FILENAME))?;
        let reference_start = RunStartFile::load(reference_dir.join(RUN_START_FILENAME))?;

        let candidate_layout = ModelLayout::from_output_dir(candidate_dir)?;
        let reference_layout = ModelLayout::from_output_dir(reference_dir)?;
        let candidate_model =
            extract_model(candidate_layout.output_file(candidate_layout.model_dir(), "lst"))?;
        let reference_model =
            extract_model(reference_layout.output_file(reference_layout.model_dir(), "lst"))?;

        let candidate_summary = get_summary(candidate_dir, None, false)?;
        let reference_summary = get_summary(reference_dir, None, false)?;

        let candidate_final_est = candidate_summary
            .final_estimation_method()
            .ok_or_else(|| anyhow!("no estimation method found in {candidate_dir:?}"))?;
        let reference_final_est = reference_summary
            .final_estimation_method()
            .ok_or_else(|| anyhow!("no estimation method found in {reference_dir:?}"))?;

        if candidate_final_est != reference_final_est {
            bail!("final estimation methods differ: {candidate_final_est} vs {reference_final_est}")
        };

        if candidate_start.dataset_hashes.blake3 != reference_start.dataset_hashes.blake3 {
            bail!("datasets differ (file hash mismatch); comparison not valid")
        }
        if !same_data_interpretation(&candidate_model, &reference_model) {
            bail!("$DATA selection or interpretation differs; comparison not valid")
        }
        if !same_input_mapping(&candidate_model, &reference_model) {
            bail!("$INPUT columns differ; comparison not valid")
        }

        let nested = tree.related_by_key(&candidate_start.model_path, &reference_start.model_path);

        let candidate_ic = candidate_summary
            .final_information_criteria()
            .ok_or_else(|| {
                anyhow!("no information criteria for final method in {candidate_dir:?}")
            })?;
        let reference_ic = reference_summary
            .final_information_criteria()
            .ok_or_else(|| {
                anyhow!("no information criteria for final method in {reference_dir:?}")
            })?;

        if candidate_ic.n_observations != reference_ic.n_observations {
            bail!("models have differing number of observations")
        }

        Ok(ModelComparison::new(&candidate_ic, &reference_ic, nested))
    }
}

/// `$DATA` controls how NONMEM reads and filters the hash-identified file.
/// The path itself is intentionally excluded because the hash already establishes
/// identity and permits equivalent files at different locations.
fn same_data_interpretation(first: &nonmem_parser::Model, second: &nonmem_parser::Model) -> bool {
    same_set(&first.data.ignore, &second.data.ignore)
        && same_set(&first.data.accept, &second.data.accept)
        && first.data.num_records == second.data.num_records
        && first.data.null_value == second.data.null_value
        && same_set(&first.data.other_options, &second.data.other_options)
}

/// `$INPUT` is positional, so two mappings are equivalent only in the same order.
fn same_input_mapping(first: &nonmem_parser::Model, second: &nonmem_parser::Model) -> bool {
    first.input_columns == second.input_columns
}

/// Order-insensitive equality for `$DATA` option lists. Needs only `PartialEq`
/// because `DataFilter` holds an `f64`, so it can't be `Ord`/`Hash`.
fn same_set<T: PartialEq>(a: &[T], b: &[T]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut matched = vec![false; b.len()];
    a.iter().all(|x| {
        match b
            .iter()
            .enumerate()
            .position(|(i, y)| !matched[i] && y == x)
        {
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
    use crate::output_files::metrics::*;

    fn parse_model(input: &str) -> nonmem_parser::Model {
        nonmem_parser::Model::parse("test.mod", input).unwrap()
    }

    #[test]
    fn input_mapping_is_order_sensitive() {
        let first = parse_model("$PROBLEM test\n$INPUT ID TIME DV\n$DATA data.csv\n");
        let second = parse_model("$PROBLEM test\n$INPUT TIME ID DV\n$DATA data.csv\n");

        assert!(!same_input_mapping(&first, &second));
    }

    #[test]
    fn data_interpretation_includes_non_filter_options() {
        let first = parse_model(
            "$PROBLEM test\n$INPUT ID TIME DV\n$DATA data.csv RECORDS=10 NULL=0 WIDE\n",
        );
        let second = parse_model(
            "$PROBLEM test\n$INPUT ID TIME DV\n$DATA data.csv RECORDS=20 NULL=0 WIDE\n",
        );

        assert!(!same_data_interpretation(&first, &second));
    }

    #[test]
    fn test_model_comparison() {
        let base = InformationCriteria::new(1000.0, 6, 320);
        let full = InformationCriteria::new(981.326, 7, 320);
        let alt = InformationCriteria::new(997.5000, 7, 320);

        let comp = ModelComparison::new(&full, &base, Some(true));
        assert!((comp.delta_ofv - -18.674).abs() < 1e-10);
        let Lrt::Computed(lrt) = comp.lrt else {
            panic!("expected a computed LRT")
        };
        assert!(lrt.p_value < 0.05);

        let comp = ModelComparison::new(&alt, &base, Some(true));
        assert!((comp.delta_ofv - -2.5).abs() < 1e-10);
        let Lrt::Computed(lrt) = comp.lrt else {
            panic!("expected a computed LRT")
        };
        assert!(lrt.p_value > 0.05);

        let comp = ModelComparison::new(&alt, &base, Some(false));
        assert!((comp.delta_ofv - -2.5).abs() < 1e-10);
        assert_eq!(comp.lrt, Lrt::NotNested);

        // Deltas are still reported when lineage can't answer nestedness.
        let comp = ModelComparison::new(&alt, &base, None);
        assert!((comp.delta_ofv - -2.5).abs() < 1e-10);
        assert_eq!(comp.lrt, Lrt::LineageUnavailable);
    }
}
