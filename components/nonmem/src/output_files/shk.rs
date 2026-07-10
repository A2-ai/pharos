use super::parsing::split_table_row;
use crate::estimation::{EstimationMethod, extract_estimation_method};
use anyhow::{Context, Result, bail};
use fs_err as fs;
use serde::Serialize;
use std::cmp::max;
use std::io::{BufRead, BufReader};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Default)]
pub struct ShkTable {
    pub method: Option<EstimationMethod>,
    pub subpop: usize,
    /// TYPE 1: ETABAR - Mean of individual ETA estimates.
    /// Should be close to zero; indicates bias in population parameter estimates
    pub etabar: Option<Vec<f64>>,
    /// TYPE 2: SE of ETABAR - Standard error of ETABAR.
    /// Used to calculate p-values for ETABAR significance test
    pub etabar_se: Option<Vec<f64>>,
    /// TYPE 3: P-values - P-values for H₀: true ETABAR = 0.
    /// Tests if population parameters are unbiased
    pub etabar_pval: Option<Vec<f64>>,
    /// TYPE 4: ETASHRINKSD(%) - ETA shrinkage (standard deviation).
    /// Key metric: % reduction in individual ETA variability
    pub eta_shrinkage_sd: Option<Vec<f64>>,
    /// TYPE 5: EPSSHRINKSD(%) - Epsilon shrinkage (standard deviation).
    /// % reduction in residual error variability; affects diagnostics.
    /// Maps to SIGMA terms.
    pub eps_shrinkage_sd: Option<Vec<f64>>,
    /// TYPE 6: EBVSHRINKSD(%) - Empirical Bayes Variance shrinkage (SD).
    /// Alternative ETA shrinkage calculation. Similar to TYPE 4
    pub ebv_shrinkage_sd: Option<Vec<f64>>,
    /// TYPE 7: N - Number of individuals.
    /// Sample size for each parameter. Same N repeated across all columns
    pub n_individuals: Option<u32>,
    /// TYPE 8: ETASHRINKVR(%) - ETA shrinkage (variance reduction).
    /// Alternative representation: 1-(1-shrinkage_SD)². Generally higher than SD
    pub eta_shrinkage_vr: Option<Vec<f64>>,
    /// TYPE 9: EBVSHRINKVR(%) - EBV shrinkage (variance reduction).
    /// EBV version of variance reduction. Similar to TYPE 8
    pub ebv_shrinkage_vr: Option<Vec<f64>>,
    /// TYPE 10: EPSSHRINKVR(%) - Epsilon shrinkage (variance reduction).
    /// Epsilon version of variance reduction. Related to TYPE 5. Maps to SIGMA terms.
    pub eps_shrinkage_vr: Option<Vec<f64>>,
    /// TYPE 11: RELATIVEINF(%) - Relative information.
    /// % of information retained (inverse of shrinkage)
    pub relative_information: Option<Vec<f64>>,
}

impl ShkTable {
    pub fn new(subpop: usize) -> Self {
        Self {
            subpop,
            ..Default::default()
        }
    }

    pub fn to_csv(&self) -> String {
        let mut lines = Vec::new();

        // Determine the maximum number of columns needed
        let max_eta_cols = self
            .etabar
            .as_ref()
            .or(self.eta_shrinkage_sd.as_ref())
            .or(self.relative_information.as_ref())
            .map(|v| v.len())
            .unwrap_or(0);

        // Create headers - assume ETA columns
        let mut headers = vec!["TYPE".to_string(), "SUBPOP".to_string()];
        for i in 1..=max_eta_cols {
            headers.push(format!("ETA({})", i));
        }
        lines.push(headers.join(","));

        // Helper to create CSV row
        let mut add_row = |type_num: u8, values: &[f64]| {
            let mut row = vec![type_num.to_string(), self.subpop.to_string()];
            row.extend(values.iter().map(|v| v.to_string()));
            // Pad with zeros if necessary to match header length
            while row.len() < headers.len() {
                row.push("0".to_string());
            }
            lines.push(row.join(","));
        };

        // Add rows for each present type
        if let Some(ref values) = self.etabar {
            add_row(1, values);
        }
        if let Some(ref values) = self.etabar_se {
            add_row(2, values);
        }
        if let Some(ref values) = self.etabar_pval {
            add_row(3, values);
        }
        if let Some(ref values) = self.eta_shrinkage_sd {
            add_row(4, values);
        }
        if let Some(ref values) = self.eps_shrinkage_sd {
            add_row(5, values);
        }
        if let Some(ref values) = self.ebv_shrinkage_sd {
            add_row(6, values);
        }
        if let Some(n) = self.n_individuals {
            // For Type 7, repeat the value across all columns
            let repeated_values = vec![n as f64; max_eta_cols];
            add_row(7, &repeated_values);
        }
        if let Some(ref values) = self.eta_shrinkage_vr {
            add_row(8, values);
        }
        if let Some(ref values) = self.ebv_shrinkage_vr {
            add_row(9, values);
        }
        if let Some(ref values) = self.eps_shrinkage_vr {
            add_row(10, values);
        }
        if let Some(ref values) = self.relative_information {
            add_row(11, values);
        }

        lines.join("\n")
    }
}

#[derive(Debug, Default, Clone)]
pub struct ShkReader;

impl ShkReader {
    pub fn parse_file(&self, path: impl AsRef<Path>) -> Result<Vec<Vec<ShkTable>>> {
        let path = path.as_ref();
        let file = fs::File::open(path)?;
        let reader = BufReader::new(file);
        let tables = self
            .parse(reader)
            .with_context(|| format!("failed to parse {}", path.display()))?;
        Ok(tables)
    }

    pub fn parse<R: BufRead>(&self, reader: R) -> Result<Vec<Vec<ShkTable>>> {
        let mut tables = Vec::new();
        let mut current_method = None;
        let mut current_tables = vec![ShkTable::new(1)];
        let mut in_table = false;
        let mut max_subpop = 1;

        macro_rules! update_tables_field {
            ($field:ident,$value:expr) => {{
                for c in &mut current_tables {
                    c.$field = $value.clone();
                }
            }};
            ($idx:expr,$field:ident,$value:expr) => {{
                current_tables[$idx - 1].$field = $value.clone();
            }};
        }

        for line in reader.lines() {
            let line = line?;
            let trimmed = line.trim();

            if trimmed.is_empty() {
                continue;
            }

            if trimmed.starts_with("TABLE NO.") {
                if in_table {
                    update_tables_field!(method, current_method);
                    tables.push(current_tables);
                    current_tables = vec![ShkTable::new(1)];
                }
                current_method = extract_estimation_method(trimmed);
                in_table = false;
                continue;
            }

            if trimmed.starts_with("TYPE") {
                in_table = true;
                continue;
            }

            // Parse data rows
            if in_table {
                let values = split_table_row(trimmed);
                if values.is_empty() {
                    continue;
                }
                if values.len() < 2 {
                    // A well-formed data row always carries at least TYPE and
                    // SUBPOP; a shorter row means a truncated/corrupt file.
                    bail!(
                        "malformed .shk data row (expected at least TYPE and SUBPOP): {trimmed:?}"
                    );
                }

                let type_number: u8 = values[0].parse()?;
                let subpop: usize = values[1].parse()?;
                max_subpop = max(max_subpop, subpop);
                while max_subpop > current_tables.len() {
                    current_tables.push(ShkTable::new(subpop));
                }

                // Parse values: skip TYPE and SUBPOP columns
                let parsed_values: Vec<f64> = values
                    .iter()
                    .skip(2)
                    .map(|x| x.parse().unwrap_or(f64::NAN))
                    .collect();

                match type_number {
                    1 => {
                        update_tables_field!(subpop, etabar, Some(parsed_values));
                    }
                    2 => {
                        update_tables_field!(subpop, etabar_se, Some(parsed_values));
                    }
                    3 => {
                        update_tables_field!(subpop, etabar_pval, Some(parsed_values));
                    }
                    4 => {
                        update_tables_field!(subpop, eta_shrinkage_sd, Some(parsed_values));
                    }
                    5 => {
                        // SIGMA-based: trim trailing zeros
                        let mut trimmed = parsed_values;
                        while trimmed.last() == Some(&0.0) {
                            trimmed.pop();
                        }
                        update_tables_field!(subpop, eps_shrinkage_sd, Some(trimmed));
                    }
                    6 => {
                        update_tables_field!(subpop, ebv_shrinkage_sd, Some(parsed_values));
                    }
                    7 => {
                        // Sample size: take first value as u32
                        let Some(&n) = parsed_values.first() else {
                            bail!("malformed .shk TYPE 7 row (missing N value): {trimmed:?}");
                        };
                        update_tables_field!(subpop, n_individuals, Some(n as u32));
                    }
                    8 => {
                        update_tables_field!(subpop, eta_shrinkage_vr, Some(parsed_values));
                    }
                    9 => {
                        update_tables_field!(subpop, ebv_shrinkage_vr, Some(parsed_values));
                    }
                    10 => {
                        // SIGMA-based: trim trailing zeros
                        let mut trimmed = parsed_values;
                        while trimmed.last() == Some(&0.0) {
                            trimmed.pop();
                        }
                        update_tables_field!(subpop, eps_shrinkage_vr, Some(trimmed));
                    }
                    11 => {
                        update_tables_field!(subpop, relative_information, Some(parsed_values));
                    }
                    _ => {
                        // Unknown type, skip
                    }
                }
            }
        }

        // Don't forget the last table
        if in_table {
            update_tables_field!(method, current_method);
            tables.push(current_tables);
        }

        Ok(tables)
    }
}

#[cfg(test)]
mod tests {
    use insta::{assert_snapshot, glob};

    use super::*;

    #[test]
    fn can_parse_shk_files() {
        use std::path::PathBuf;
        let test_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_data/shk");
        glob!(test_dir, "*.shk", |path| {
            let reader = ShkReader;
            let result = reader.parse_file(path).unwrap();
            if path.file_name().unwrap().to_string_lossy().contains("3068") {
                assert_eq!(result[0].len(), 2);
                let mut snap = result[0][0].to_csv();
                snap.push('\n');
                snap.push('\n');
                snap.push_str(&result[0][1].to_csv());
                assert_snapshot!(snap);
            } else {
                assert_snapshot!(result[0][0].to_csv());
            }
        });
    }

    #[test]
    fn can_parse_shk_files_with_multi_methods() {
        use std::path::PathBuf;
        let test_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_data/shk");
        glob!(test_dir, "*.shk", |path| {
            let reader = ShkReader;
            let result = reader.parse_file(path).unwrap();
            if path
                .file_name()
                .unwrap()
                .to_string_lossy()
                .contains("itsimp")
            {
                assert_eq!(result.len(), 2);
                let mut snap = result[0][0].to_csv();
                snap.push('\n');
                snap.push('\n');
                snap.push_str(&result[1][0].to_csv());
                assert_snapshot!(snap);
            } else {
                assert_snapshot!(result[0][0].to_csv());
            }
        });
    }

    #[test]
    fn semantic_structure_works() {
        use std::path::PathBuf;
        let test_file = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_data/shk/bql.shk");
        let reader = ShkReader;
        let result = reader.parse_file(test_file).unwrap();
        let table = &result[0][0];

        // Test semantic access
        assert_eq!(table.n_individuals, Some(193));
        assert!(table.eta_shrinkage_sd.is_some());
        assert_eq!(table.eta_shrinkage_sd.as_ref().unwrap().len(), 3); // 3 ETA parameters

        // Test epsilon shrinkage trimming - should only have 1 meaningful value
        assert!(table.eps_shrinkage_sd.is_some());
        assert_eq!(table.eps_shrinkage_sd.as_ref().unwrap().len(), 1); // Only 1 SIGMA parameter
        assert_eq!(table.eps_shrinkage_sd.as_ref().unwrap()[0], 9.70268);

        // Test that ETA-based metrics have full length
        assert_eq!(table.etabar.as_ref().unwrap().len(), 3);
        assert_eq!(table.relative_information.as_ref().unwrap().len(), 3);
    }

    #[test]
    fn truncated_rows_error_instead_of_panicking() {
        use std::io::Cursor;
        let reader = ShkReader;
        let cases = [
            // one-token data row (truncated mid-write)
            "TABLE NO. 1\nTYPE SUBPOP\n1\n",
            // TYPE 7 (N) row missing its data column
            "TABLE NO. 1\nTYPE SUBPOP\n7 1\n",
        ];
        for case in cases {
            assert!(
                reader.parse(Cursor::new(case)).is_err(),
                "expected error, not panic, for: {case:?}"
            );
        }
    }
}
