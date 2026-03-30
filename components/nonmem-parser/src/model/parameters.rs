use std::collections::BTreeMap;

use crate::comments::CommentType;
use anyhow::bail;

use crate::ast::{BlockStructure, OmegaSigmaBlock, OmegaSigmaParam};
use crate::comments::{ParameterOrdering, parse_omega_param, parse_sigma_param, parse_theta_param};

use super::Model;

const OMEGA: &str = "OMEGA";
const SIGMA: &str = "SIGMA";
const ETA: &str = "ETA";
const EPS: &str = "EPS";

impl Model {
    /// Iterate over OMEGA parameters in specified order, yielding (param_name, eta_label, parameter)
    /// param_name is OMEGA(i,j), eta_label is ETAj:ETAi or ETAi for OMEGA(i,i)
    pub fn get_omega_parameters(
        &self,
        ordering: ParameterOrdering,
    ) -> anyhow::Result<Vec<(String, String, &OmegaSigmaParam)>> {
        get_block_parameter_names(&self.omega_blocks, ordering, OMEGA, ETA)
    }

    /// Iterate over SIGMA parameters in specified order, yielding (param_name, eps_label, parameter)
    /// param_name is SIGMA(i,j), eps_label is EPSj:EPSi or EPSi for SIGMA(i,i)
    pub fn get_sigma_parameters(
        &self,
        ordering: ParameterOrdering,
    ) -> anyhow::Result<Vec<(String, String, &OmegaSigmaParam)>> {
        get_block_parameter_names(&self.sigma_blocks, ordering, SIGMA, EPS)
    }

    /// Build a map of parameter coordinate names to parsed comment names.
    ///
    /// For each theta: `"THETA1" → Some("TVCL")` (if comment parses)
    /// For each omega/sigma: `"OMEGA(1,1)" → Some("OM1 (TVCL)")` (if comment parses)
    pub fn get_parameter_names(
        &self,
        comment_type: CommentType,
    ) -> anyhow::Result<BTreeMap<String, Option<String>>> {
        let mut parameter_names = BTreeMap::new();

        // Add THETA parameter names
        for (i, param) in self.thetas.iter().enumerate() {
            let name = param
                .comment
                .as_deref()
                .and_then(|c| parse_theta_param(c, comment_type))
                .and_then(|parsed| parsed.name());
            parameter_names.insert(format!("THETA{}", i + 1), name);
        }

        // Add OMEGA parameter names (RowMajor to match EXT file order)
        let omega_params = self.get_omega_parameters(ParameterOrdering::RowMajor)?;
        for (ext_name, _eta_label, param) in omega_params {
            let name = param
                .comment
                .as_deref()
                .and_then(|c| parse_omega_param(c, comment_type))
                .and_then(|parsed| parsed.name());
            parameter_names.insert(ext_name, name);
        }

        // Add SIGMA parameter names (RowMajor to match EXT file order)
        let sigma_params = self.get_sigma_parameters(ParameterOrdering::RowMajor)?;
        for (ext_name, _eps_label, param) in sigma_params {
            let name = param
                .comment
                .as_deref()
                .and_then(|c| parse_sigma_param(c, comment_type))
                .and_then(|parsed| parsed.name());
            parameter_names.insert(ext_name, name);
        }

        Ok(parameter_names)
    }
}

/// Iterate over parameter blocks in specified order, yielding (param_name, raneff_label, parameter)
fn get_block_parameter_names<'a>(
    blocks: &'a [OmegaSigmaBlock],
    ordering: ParameterOrdering,
    param_prefix: &str,
    raneff_prefix: &str,
) -> anyhow::Result<Vec<(String, String, &'a OmegaSigmaParam)>> {
    let mut results = Vec::new();
    let mut base_counter = 1;

    for (block_index, block) in blocks.iter().enumerate() {
        match &block.structure {
            BlockStructure::Diagonal => {
                for (param_idx, param) in block.parameters.iter().enumerate() {
                    let num = base_counter + param_idx;
                    let param_name = format!("{param_prefix}({num},{num})");
                    let raneff_label = format!("{raneff_prefix}{num}");
                    results.push((param_name, raneff_label, param));
                }
                base_counter += block.parameters.len();
            }
            BlockStructure::Block { size } => {
                for (storage_idx, row, col) in ordering.get_indexed_coordinates(*size) {
                    if storage_idx >= block.parameters.len() {
                        break;
                    }

                    let param = &block.parameters[storage_idx];
                    let param_row = base_counter + row;
                    let param_col = base_counter + col;
                    let param_name = format!("{param_prefix}({param_row},{param_col})");
                    let raneff_label = if row == col {
                        format!("{raneff_prefix}{param_row}")
                    } else {
                        format!("{raneff_prefix}{param_col}:{raneff_prefix}{param_row}")
                    };
                    results.push((param_name, raneff_label, param));
                }
                base_counter += size;
            }
            BlockStructure::BlockSame { size, repeats } => {
                // Find reference block - search backwards for most recent Block with matching size
                let mut reference_block = None;
                for i in (0..block_index).rev() {
                    if let BlockStructure::Block { size: ref_size } = &blocks[i].structure
                        && *ref_size == *size
                    {
                        reference_block = Some(&blocks[i]);
                        break;
                    }
                }

                let Some(ref_block) = reference_block else {
                    bail!(
                        "BlockSame {{size: {size}}} found but no previous Block {{size: {size}}} to reference"
                    )
                };

                // BlockSame repeats the block `repeats` times
                for _ in 0..*repeats {
                    for (storage_idx, row, col) in ordering.get_indexed_coordinates(*size) {
                        if storage_idx >= ref_block.parameters.len() {
                            break;
                        }

                        let param = &ref_block.parameters[storage_idx];
                        let param_row = base_counter + row;
                        let param_col = base_counter + col;
                        let param_name = format!("{param_prefix}({param_row},{param_col})");
                        let raneff_label = if row == col {
                            format!("{raneff_prefix}{param_row}")
                        } else {
                            format!("{raneff_prefix}{param_col}:{raneff_prefix}{param_row}")
                        };
                        results.push((param_name, raneff_label, param));
                    }
                    base_counter += size;
                }
            }
        }
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use crate::comments::{CommentType, ParameterOrdering};
    use crate::model::Model;

    fn parse_model(input: &str) -> Model {
        let (model, diagnostics) = Model::parse(input).unwrap();
        assert!(
            diagnostics.is_empty(),
            "unexpected diagnostics: {diagnostics:?}"
        );
        model
    }

    fn load_model(name: &str) -> Model {
        let path = format!("{}/test_data/{name}", env!("CARGO_MANIFEST_DIR"));
        let input = fs_err::read_to_string(&path).unwrap();
        parse_model(&input)
    }

    // --- ParameterOrdering::get_coordinates ---

    #[test]
    fn get_coordinates_row_major() {
        let coords = ParameterOrdering::RowMajor.get_coordinates(3);
        assert_eq!(coords, vec![(0, 0), (1, 0), (1, 1), (2, 0), (2, 1), (2, 2)]);
    }

    #[test]
    fn get_coordinates_column_major() {
        let coords = ParameterOrdering::ColumnMajor.get_coordinates(3);
        assert_eq!(coords, vec![(0, 0), (1, 0), (2, 0), (1, 1), (2, 1), (2, 2)]);
    }

    // --- get_omega_parameters ---

    #[test]
    fn omega_parameters_row_major() {
        let model = load_model("everything.mod");
        let params = model
            .get_omega_parameters(ParameterOrdering::RowMajor)
            .unwrap();
        let pairs: Vec<(String, String)> = params
            .into_iter()
            .map(|(name, label, _)| (name, label))
            .collect();
        insta::assert_debug_snapshot!(pairs);
    }

    #[test]
    fn omega_parameters_column_major() {
        let model = load_model("everything.mod");
        let params = model
            .get_omega_parameters(ParameterOrdering::ColumnMajor)
            .unwrap();
        let pairs: Vec<(String, String)> = params
            .into_iter()
            .map(|(name, label, _)| (name, label))
            .collect();
        insta::assert_debug_snapshot!(pairs);
    }

    // --- get_sigma_parameters ---

    #[test]
    fn sigma_parameters_row_major() {
        let model = load_model("everything.mod");
        let params = model
            .get_sigma_parameters(ParameterOrdering::RowMajor)
            .unwrap();
        let pairs: Vec<(String, String)> = params
            .into_iter()
            .map(|(name, label, _)| (name, label))
            .collect();
        insta::assert_debug_snapshot!(pairs);
    }

    // --- get_parameter_names ---

    #[test]
    fn parameter_names_type1_comments() {
        let model = load_model("comments/type1.mod");
        let names = model.get_parameter_names(CommentType::Type1).unwrap();
        insta::assert_debug_snapshot!(names);
    }

    #[test]
    fn parameter_names_no_type1_comments() {
        let model = load_model("everything.mod");
        let names = model.get_parameter_names(CommentType::Type1).unwrap();
        // everything.mod has no type1-formatted comments, so all values should be None
        for (key, value) in &names {
            assert!(value.is_none(), "expected None for {key}, got {value:?}");
        }
    }

    #[test]
    fn block_same_advances_parameter_positions() {
        let input = r#"
$PROBLEM same block indexing
$OMEGA BLOCK(1)
0.1
$OMEGA BLOCK(1) SAME
$OMEGA BLOCK(1)
0.2
"#;

        let model = parse_model(input);
        let omega_names = model
            .get_omega_parameters(ParameterOrdering::RowMajor)
            .unwrap();

        let names: Vec<_> = omega_names
            .into_iter()
            .map(|(param_name, _eta_label, _param)| param_name)
            .collect();

        assert_eq!(
            names,
            vec![
                "OMEGA(1,1)".to_string(),
                "OMEGA(2,2)".to_string(),
                "OMEGA(3,3)".to_string(),
            ]
        );
    }
}
