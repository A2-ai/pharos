use anyhow::Result;
use fs_err as fs;
use serde::Serialize;
use std::io::{BufRead, BufReader};
use std::path::Path;

use super::parsing::{self, ParseContext};
use crate::estimation::{EstimationMethod, extract_estimation_method};

/// A single row of parameter estimates
#[derive(Debug, Clone)]
pub struct RawShkRow {
    pub type_num: u8,
    pub subpop: usize,
    pub values: Vec<f64>,
}

/// Represents a single estimation table from a NONMEM .ext file
#[derive(Debug, Clone)]
pub struct RawShkTable {
    /// Estimation method (e.g., "First Order Conditional Estimation", "Iterative Two Stage")
    pub method: Option<EstimationMethod>,
    /// Parameter names from the TYPE header line
    pub parameters: Vec<String>,
    /// Rows of parameter values
    pub rows: Vec<RawShkRow>,
}

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

    /// Set field based on type number from .shk file
    pub fn set_field_by_type_num(&mut self, type_num: u8, values: Vec<f64>) {
        match type_num {
            1 => self.etabar = Some(values),
            2 => self.etabar_se = Some(values),
            3 => self.etabar_pval = Some(values),
            4 => self.eta_shrinkage_sd = Some(values),
            5 => {
                // SIGMA-based: trim trailing zeros
                let mut trimmed = values;
                while trimmed.last() == Some(&0.0) {
                    trimmed.pop();
                }
                self.eps_shrinkage_sd = Some(trimmed);
            }
            6 => self.ebv_shrinkage_sd = Some(values),
            7 => {
                // Sample size: take first value as u32
                if let Some(&first_value) = values.first() {
                    self.n_individuals = Some(first_value as u32);
                }
            }
            8 => self.eta_shrinkage_vr = Some(values),
            9 => self.ebv_shrinkage_vr = Some(values),
            10 => {
                // SIGMA-based: trim trailing zeros
                let mut trimmed = values;
                while trimmed.last() == Some(&0.0) {
                    trimmed.pop();
                }
                self.eps_shrinkage_vr = Some(trimmed);
            }
            11 => self.relative_information = Some(values),
            _ => {
                // Unknown type, skip
            }
        }
    }

    /// Get field values and type number pairs for CSV generation
    pub fn get_type_field_pairs(&self) -> Vec<(u8, Vec<f64>)> {
        let mut pairs = Vec::new();

        if let Some(ref values) = self.etabar {
            pairs.push((1, values.clone()));
        }
        if let Some(ref values) = self.etabar_se {
            pairs.push((2, values.clone()));
        }
        if let Some(ref values) = self.etabar_pval {
            pairs.push((3, values.clone()));
        }
        if let Some(ref values) = self.eta_shrinkage_sd {
            pairs.push((4, values.clone()));
        }
        if let Some(ref values) = self.eps_shrinkage_sd {
            pairs.push((5, values.clone()));
        }
        if let Some(ref values) = self.ebv_shrinkage_sd {
            pairs.push((6, values.clone()));
        }
        if let Some(n) = self.n_individuals {
            // For Type 7, we need to determine how many columns to repeat across
            let max_eta_cols = self
                .etabar
                .as_ref()
                .or(self.eta_shrinkage_sd.as_ref())
                .or(self.relative_information.as_ref())
                .map(|v| v.len())
                .unwrap_or(0);
            let repeated_values = vec![n as f64; max_eta_cols];
            pairs.push((7, repeated_values));
        }
        if let Some(ref values) = self.eta_shrinkage_vr {
            pairs.push((8, values.clone()));
        }
        if let Some(ref values) = self.ebv_shrinkage_vr {
            pairs.push((9, values.clone()));
        }
        if let Some(ref values) = self.eps_shrinkage_vr {
            pairs.push((10, values.clone()));
        }
        if let Some(ref values) = self.relative_information {
            pairs.push((11, values.clone()));
        }

        pairs
    }

    pub fn to_csv(&self) -> String {
        let mut lines = Vec::new();
        let type_field_pairs = self.get_type_field_pairs();

        if type_field_pairs.is_empty() {
            return String::new();
        }

        // Determine the maximum number of columns needed
        let max_eta_cols = type_field_pairs
            .iter()
            .map(|(_, values)| values.len())
            .max()
            .unwrap_or(0);

        // Create headers - assume ETA columns
        let mut headers = vec!["TYPE".to_string(), "SUBPOP".to_string()];
        for i in 1..=max_eta_cols {
            headers.push(format!("ETA({})", i));
        }
        lines.push(headers.join(","));

        // Add rows for each type
        for (type_num, values) in type_field_pairs {
            let mut row = vec![type_num.to_string(), self.subpop.to_string()];
            row.extend(values.iter().map(|v| v.to_string()));
            // Pad with zeros if necessary to match header length
            while row.len() < headers.len() {
                row.push("0".to_string());
            }
            lines.push(row.join(","));
        }

        lines.join("\n")
    }
}

// Converts RawShkTable into vec of ShkTable based
// on subpopulation.
impl From<RawShkTable> for Vec<ShkTable> {
    fn from(raw: RawShkTable) -> Self {
        // Find all unique subpopulation numbers
        let mut subpops: Vec<usize> = raw.rows.iter().map(|row| row.subpop).collect();
        subpops.sort_unstable();
        subpops.dedup();

        // Create one ShkTable per subpopulation
        let mut tables: Vec<ShkTable> = subpops
            .iter()
            .map(|&subpop| {
                let mut table = ShkTable::new(subpop);
                table.method = raw.method;
                table
            })
            .collect();

        // Process each raw row and populate the appropriate semantic fields
        for row in &raw.rows {
            if let Some(table) = tables.iter_mut().find(|t| t.subpop == row.subpop) {
                table.set_field_by_type_num(row.type_num, row.values.clone());
            }
        }

        tables
    }
}

#[derive(Debug, Clone)]
pub struct ShkReader {
    only_last: bool,
    only_method: Option<EstimationMethod>,
}

impl Default for ShkReader {
    fn default() -> Self {
        ShkReader {
            only_last: false,
            only_method: None,
        }
    }
}

impl ShkReader {
    /// Only keep the table corresponding to the given method.
    /// Exclusive of `only_last`: it will be set to false if this method is called
    pub fn only_method(mut self, method: EstimationMethod) -> Self {
        self.only_method = Some(method);
        self.only_last = false;
        self
    }

    /// Keep all tables from the input file.
    /// This will disable `only_last` and `only_method`
    pub fn keep_all_tables(mut self) -> Self {
        self.only_last = false;
        self.only_method = None;
        self
    }

    /// Only keep the last table.
    /// This is the default.
    pub fn only_last(mut self) -> Self {
        self.only_last = true;
        self.only_method = None;
        self
    }

    /// Parse a .shk file and return semantic table structure
    pub fn parse_file_semantic(&self, path: impl AsRef<Path>) -> Result<Vec<Vec<ShkTable>>> {
        let raw_tables = self.parse_file(path)?;
        Ok(raw_tables.into_iter().map(|raw| raw.into()).collect())
    }

    pub fn parse_file(&self, path: impl AsRef<Path>) -> Result<Vec<RawShkTable>> {
        let path = path.as_ref();
        let file = fs::File::open(path)?;
        let reader = BufReader::new(file);
        let tables = self.parse(reader)?;
        Ok(tables)
    }

    pub fn parse<R: BufRead>(&self, mut reader: R) -> Result<Vec<RawShkTable>> {
        // Read entire content into memory
        let mut content = String::new();
        reader.read_to_string(&mut content)?;

        let lines: Vec<&str> = content.lines().collect();
        let table_positions = parsing::find_table_positions(&lines);

        if table_positions.is_empty() {
            return Ok(Vec::new());
        }

        let context = ParseContext {
            only_method: self.only_method,
            only_last: self.only_last,
        };
        let lines_to_parse = parsing::select_lines_to_parse(&lines, &table_positions, &context);

        if lines_to_parse.is_empty() {
            return Ok(Vec::new());
        }

        let mut tables = Vec::new();
        let mut current_method = None;
        let mut current_parameters = None;
        let mut current_rows = Vec::new();
        let mut in_table = false;

        for line in lines_to_parse {
            let trimmed = line.trim();

            if trimmed.is_empty() {
                continue;
            }

            if trimmed.starts_with("TABLE NO.") {
                // Save previous table if exists
                if let Some(params) = current_parameters.take() {
                    tables.push(RawShkTable {
                        method: current_method,
                        parameters: params,
                        rows: std::mem::take(&mut current_rows),
                    });
                }
                // Extract method from TABLE NO. line
                current_method = extract_estimation_method(trimmed);
                in_table = false;
                continue;
            }

            if trimmed.starts_with("TYPE") {
                // Extract parameters from TYPE header line, skip TYPE and SUBPOP columns - always the first two
                let all_params: Vec<String> =
                    trimmed.split_whitespace().map(|s| s.to_string()).collect();
                current_parameters = Some(all_params.into_iter().skip(2).collect());
                in_table = true;
                continue;
            }

            // Parse data rows directly into RawShkRow
            if in_table {
                let values = trimmed.split_whitespace().collect::<Vec<_>>();
                if values.len() >= 3 {
                    let type_num: u8 = values[0].parse()?;
                    let subpop: usize = values[1].parse()?;

                    // Parse remaining values: skip TYPE and SUBPOP columns
                    // columns
                    let parsed_values: Vec<f64> = values
                        .iter()
                        .skip(2)
                        .map(|x| x.parse().unwrap_or(f64::NAN))
                        .collect();

                    current_rows.push(RawShkRow {
                        type_num,
                        subpop,
                        values: parsed_values,
                    });
                }
            }
        }

        // Don't forget the last table
        if let Some(params) = current_parameters {
            tables.push(RawShkTable {
                method: current_method,
                parameters: params,
                rows: current_rows,
            });
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
            let reader = ShkReader::default();
            let result = reader.parse_file(path).unwrap();
            if path
                .file_name()
                .unwrap()
                .to_string_lossy()
                .contains("itsimp")
            {
                assert_eq!(result.len(), 2);
                assert_snapshot!(format!("{:#?}", result));
            } else {
                assert_snapshot!(format!("{:#?}", result));
            }
        });
    }

    #[test]
    fn can_parse_shk_files_semantically() {
        use std::path::PathBuf;
        let test_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_data/shk");
        glob!(test_dir, "*.shk", |path| {
            let reader = ShkReader::default();
            let result = reader.parse_file_semantic(path).unwrap();
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
    fn semantic_structure_works() {
        use std::path::PathBuf;
        let test_file = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_data/shk/bql.shk");
        let reader = ShkReader::default();
        let result = reader.parse_file_semantic(test_file).unwrap();
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
    fn can_filter_by_method() {
        use std::path::PathBuf;
        let test_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_data/shk");
        glob!(test_dir, "*.shk", |path| {
            let filename = path.file_stem().unwrap().to_string_lossy();

            // Test Its method filtering
            let reader = ShkReader::default().only_method(EstimationMethod::Its);
            let result = reader.parse_file(path).unwrap();
            assert_snapshot!(format!("{}_its_only", filename), format!("{:#?}", result));

            // Test ImportanceSampling method filtering
            let reader = ShkReader::default().only_method(EstimationMethod::Imp);
            let result = reader.parse_file(path).unwrap();
            assert_snapshot!(
                format!("{}_importance_only", filename),
                format!("{:#?}", result)
            );
        });
    }

    #[test]
    fn can_keep_all_tables() {
        use std::path::PathBuf;
        let test_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_data/shk");
        glob!(test_dir, "*.shk", |path| {
            let filename = path.file_stem().unwrap().to_string_lossy();
            let reader = ShkReader::default().keep_all_tables();
            let result = reader.parse_file(path).unwrap();
            assert_snapshot!(format!("{}_all_tables", filename), format!("{:#?}", result));
        });
    }

    #[test]
    fn can_keep_only_last_table() {
        use std::path::PathBuf;
        let test_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_data/shk");
        glob!(test_dir, "*.shk", |path| {
            let filename = path.file_stem().unwrap().to_string_lossy();
            let reader = ShkReader::default().only_last();
            let result = reader.parse_file(path).unwrap();
            assert_snapshot!(format!("{}_last_only", filename), format!("{:#?}", result));
        });
    }
}
