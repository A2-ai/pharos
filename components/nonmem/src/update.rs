//! General model-update primitives: set a model's initial estimates from a
//! prior run's `.ext` file (in place or on a parsed `Model`), and release or
//! re-fix individual thetas.
//!
//! This is the layer SCM retries build on ("start the retry from where the
//! previous attempt left off"), but it is deliberately not SCM-specific

use std::collections::HashMap;
use std::io::Write;
use std::path::Path;

use anyhow::{Context, Result, bail};
use fs_err as fs;
use nonmem_parser::Model;

use crate::copy::UpdateType;
use crate::output_files::ext::{ExtReader, FINAL_ESTIMATES_ITERATION};

fn wants(update: &[UpdateType], t: UpdateType) -> bool {
    update.contains(&t) || update.contains(&UpdateType::All)
}

/// Read parameter estimates from a `.ext` file into a map keyed by parameter
/// name (`THETA1`, `OMEGA(1,1)`, ...), including only the parameter types
/// selected by `update`.
///
/// With `allow_partial = false` only the final-estimates row is used; a
/// parameter whose value cannot be read comes back as NaN so the caller can
/// decide how to react (this matches the strict behavior `copy_model` relies
/// on).
///
/// With `allow_partial = true`, a missing final-estimates row falls back to
/// the last regular iteration row — the estimates where the run left off —
/// and non-finite values are silently dropped. This is what lets a failed fit
/// be retried from its last known position.
pub fn read_ext_estimates(
    ext_path: impl AsRef<Path>,
    update: &[UpdateType],
    allow_partial: bool,
) -> Result<HashMap<String, f64>> {
    let ext_path = ext_path.as_ref();
    let tables = ExtReader::default()
        .parse_file(ext_path)
        .with_context(|| format!("failed to read estimates from {}", ext_path.display()))?;

    let Some(table) = tables.last() else {
        bail!("No parameter estimates found in {}", ext_path.display());
    };

    let final_row = table
        .rows
        .iter()
        .find(|row| row.iteration == FINAL_ESTIMATES_ITERATION);

    let row = match final_row {
        Some(row) => row,
        None if allow_partial => {
            // The run never reached final estimates; use the last iteration.
            match table.rows.iter().rfind(|r| r.iteration >= 0) {
                Some(row) => row,
                None => bail!(
                    "No usable estimate rows found in {} (no final estimates and no iterations)",
                    ext_path.display()
                ),
            }
        }
        // Strict mode mirrors the historical behavior: missing values are
        // reported as NaN for the caller to reject with context.
        None => {
            return Ok(named_values(table, &[], update, f64::NAN));
        }
    };

    let mut estimates = named_values(table, &row.values, update, f64::NAN);
    if allow_partial {
        estimates.retain(|_, v| v.is_finite());
    }
    Ok(estimates)
}

/// Zip the table's parameter names with a row of values, keeping only the
/// requested parameter types. Missing values become `missing`.
fn named_values(
    table: &crate::output_files::ext::EstimationTable,
    values: &[f64],
    update: &[UpdateType],
    missing: f64,
) -> HashMap<String, f64> {
    let mut out = HashMap::new();
    for (i, name) in table.parameters.iter().enumerate() {
        let included = (name.starts_with("THETA") && wants(update, UpdateType::Theta))
            || (name.starts_with("OMEGA") && wants(update, UpdateType::Omega))
            || (name.starts_with("SIGMA") && wants(update, UpdateType::Sigma));
        if !included {
            continue;
        }
        out.insert(name.clone(), values.get(i).copied().unwrap_or(missing));
    }
    out
}

/// Update a model's initial estimates in place from a run's `.ext` file — the
/// `update_inits` operation. The model file is rewritten; everything except
/// the updated estimate tokens is preserved byte-for-byte.
///
/// `allow_partial` extends this to unfinished/failed runs by falling back to
/// the last iteration row (see [`read_ext_estimates`]).
pub fn update_model_estimates(
    model_path: impl AsRef<Path>,
    ext_path: impl AsRef<Path>,
    update: &[UpdateType],
    allow_partial: bool,
) -> Result<()> {
    let model_path = model_path.as_ref();
    let estimates = read_ext_estimates(ext_path.as_ref(), update, allow_partial)?;

    if !allow_partial {
        for (name, value) in &estimates {
            if !value.is_finite() {
                bail!(
                    "Invalid estimate found for {name} in {}, the run may not have finished.",
                    ext_path.as_ref().display()
                );
            }
        }
    }

    let mut model = Model::parse(model_path, &fs::read_to_string(model_path)?)?;
    model.update_initial_estimates(&estimates, None, None, &[]);

    let mut f = fs::File::create(model_path)?;
    f.write_all(model.model_content().as_bytes())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_data(rel: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("test_data")
            .join(rel)
    }

    #[test]
    fn strict_read_returns_nan_for_unfinished_run() {
        let estimates = read_ext_estimates(
            test_data("copy/still_running.ext"),
            &[UpdateType::All],
            false,
        )
        .unwrap();
        assert!(!estimates.is_empty());
        assert!(estimates.values().any(|v| !v.is_finite()));
    }

    #[test]
    fn partial_read_falls_back_to_last_iteration() {
        let estimates = read_ext_estimates(
            test_data("copy/still_running.ext"),
            &[UpdateType::All],
            true,
        )
        .unwrap();
        assert!(
            !estimates.is_empty(),
            "expected estimates from the last iteration row"
        );
        assert!(estimates.values().all(|v| v.is_finite()));
        assert!(estimates.keys().any(|k| k.starts_with("THETA")));
    }

    #[test]
    fn read_filters_by_update_type() {
        let all = read_ext_estimates(
            test_data("copy/still_running.ext"),
            &[UpdateType::All],
            true,
        )
        .unwrap();
        let thetas_only = read_ext_estimates(
            test_data("copy/still_running.ext"),
            &[UpdateType::Theta],
            true,
        )
        .unwrap();
        assert!(thetas_only.keys().all(|k| k.starts_with("THETA")));
        assert!(thetas_only.len() < all.len());
    }

    #[test]
    fn finished_run_prefers_final_estimates() {
        // ext fixture with a final-estimates row
        let path = test_data("ext/bql.ext");
        let strict = read_ext_estimates(&path, &[UpdateType::All], false).unwrap();
        let partial = read_ext_estimates(&path, &[UpdateType::All], true).unwrap();
        for (name, value) in &partial {
            assert_eq!(strict.get(name), Some(value), "mismatch for {name}");
        }
    }

    #[test]
    fn update_model_estimates_rewrites_in_place() {
        let dir = tempfile::tempdir().unwrap();
        let model_path = dir.path().join("run001.mod");
        fs::write(
            &model_path,
            "\
$PROBLEM test
$INPUT ID TIME DV
$DATA data.csv
$THETA (0, 1.0)   ; TVCL
$THETA 0.5 FIX    ; FIXED ONE
$OMEGA 0.04
$SIGMA 0.01
$EST METHOD=1
",
        )
        .unwrap();

        // Build a small finished ext beside it
        let ext_path = dir.path().join("run001.ext");
        fs::write(
            &ext_path,
            "TABLE NO.     1: First Order Conditional Estimation with Interaction\n\
 ITERATION    THETA1       THETA2       OMEGA(1,1)   SIGMA(1,1)   OBJ\n\
            0  1.00000E+00  5.00000E-01  4.00000E-02  1.00000E-02  1000\n\
  -1000000000  2.34000E+00  5.00000E-01  9.90000E-02  2.00000E-02  900\n",
        )
        .unwrap();

        update_model_estimates(&model_path, &ext_path, &[UpdateType::All], false).unwrap();
        let content = fs::read_to_string(&model_path).unwrap();
        assert!(content.contains("2.34"), "content:\n{content}");
        assert!(content.contains("0.099"), "content:\n{content}");
        // Fixed theta untouched
        assert!(content.contains("0.5 FIX"), "content:\n{content}");
        // Comments preserved
        assert!(content.contains("; TVCL"), "content:\n{content}");
    }

    #[test]
    fn update_model_estimates_partial_uses_last_iteration() {
        let dir = tempfile::tempdir().unwrap();
        let model_path = dir.path().join("run001.mod");
        fs::write(
            &model_path,
            "\
$PROBLEM test
$INPUT ID TIME DV
$DATA data.csv
$THETA (0, 1.0)
$OMEGA 0.04
$SIGMA 0.01
$EST METHOD=1
",
        )
        .unwrap();

        // Failed run: iterations but no final-estimates row
        let ext_path = dir.path().join("run001.ext");
        fs::write(
            &ext_path,
            "TABLE NO.     1: First Order Conditional Estimation with Interaction\n\
 ITERATION    THETA1       OMEGA(1,1)   SIGMA(1,1)   OBJ\n\
            0  1.00000E+00  4.00000E-02  1.00000E-02  1000\n\
            5  1.77000E+00  6.00000E-02  1.50000E-02  950\n",
        )
        .unwrap();

        // Strict mode refuses
        let err =
            update_model_estimates(&model_path, &ext_path, &[UpdateType::All], false).unwrap_err();
        assert!(
            err.to_string().contains("may not have finished"),
            "got: {err}"
        );

        // Partial mode picks up iteration 5
        update_model_estimates(&model_path, &ext_path, &[UpdateType::All], true).unwrap();
        let content = fs::read_to_string(&model_path).unwrap();
        assert!(content.contains("1.77"), "content:\n{content}");
        assert!(content.contains("0.06"), "content:\n{content}");
    }
}
