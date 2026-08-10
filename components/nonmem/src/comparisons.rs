use anyhow::{Result as AnyhowResult, anyhow, bail};
use serde::{Deserialize, Serialize};
use statrs::distribution::{ChiSquared, ContinuousCDF};
use std::path::Path;

use crate::LineageTree;
use crate::metrics::InformationCriteria;
use crate::model_resolution::ModelLayout;
use crate::output_files::{get_summary, lst::extract_model};
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
    /// 2. Same observations: identical dataset contents (the recorded file
    ///    hash), equivalent `$DATA` interpretation/selection, and the same
    ///    ordered `$INPUT` mapping
    /// 3. Same number of observations
    /// Computes whether the models are nested for LRT
    ///
    /// Nestedness is resolved against `tree`, which the caller builds from the
    /// appropriate project root (`LineageTree::from_project` for CWD discovery,
    /// `LineageTree::from_project_root` for an explicitly resolved root). Since
    /// the tree carries its own root, this never consults global config state.
    pub fn compare_runs<P: AsRef<Path>>(
        first_dir: P,
        second_dir: P,
        tree: &LineageTree,
    ) -> AnyhowResult<Self> {
        let first_dir = first_dir.as_ref();
        let second_dir = second_dir.as_ref();

        // Loaded once up front: used for the dataset-hash guard and to
        // resolve each run to its model for the nestedness check.
        let first_start = RunStartFile::load(first_dir.join(RUN_START_FILENAME))?;
        let second_start = RunStartFile::load(second_dir.join(RUN_START_FILENAME))?;

        // Models are extracted from the .lst (the control stream as run),
        // located via each run's ModelLayout.
        let first_model = extract_model(lst_path(first_dir)?)?;
        let second_model = extract_model(lst_path(second_dir)?)?;

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
        // to be valid. The recorded hash establishes that the source data is
        // identical even if each run used a different path. The control-stream
        // options below establish that NONMEM interpreted it the same way.
        if first_start.dataset_hashes.blake3 != second_start.dataset_hashes.blake3 {
            bail!("datasets differ (file hash mismatch); comparison not valid")
        }
        if !same_data_interpretation(&first_model, &second_model) {
            bail!("$DATA selection or interpretation differs; comparison not valid")
        }
        if !same_input_mapping(&first_model, &second_model) {
            bail!("$INPUT columns differ; comparison not valid")
        }

        // Nestedness from lineage. The stored `model_path` is relative to the
        // project root, so it's already a tree key.
        let nested = tree.is_related(&first_start.model_path, &second_start.model_path)?;

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

/// The `.lst` path for a run directory, resolved via its `ModelLayout` so a
/// configurable output-dir name doesn't break the lookup.
fn lst_path(dir: &Path) -> AnyhowResult<std::path::PathBuf> {
    let layout = ModelLayout::from_output_dir(dir)?;
    Ok(layout.output_file(layout.model_dir(), "lst"))
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
    use crate::metrics::*;

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
