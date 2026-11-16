use std::collections::BTreeMap;

use crate::comments::{
    CommentType, Transform, parse_omega_param, parse_sigma_param, parse_theta_param,
};
use anyhow::bail;

use crate::ast::{BlockStructure, OmegaSigmaBlock, OmegaSigmaParam};

use super::Model;

const OMEGA: &str = "OMEGA";
const SIGMA: &str = "SIGMA";
const ETA: &str = "ETA";
const EPS: &str = "EPS";

pub struct OmegaSigmaEntry {
    pub param_name: String,
    pub raneff_label: String,
    pub parameter: OmegaSigmaParam,
    pub block_fixed: bool,
}

/// What the control stream's comments say about one parameter.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ParameterInfo {
    pub label: Option<String>,
    pub transform: Option<Transform>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParameterOrdering {
    /// Row-major ordering used in EXT files: (1,1), (2,1), (2,2), (3,1), (3,2), (3,3)
    RowMajor,
    /// Column-major ordering used in GRD files: (1,1), (2,1), (3,1), (2,2), (3,2), (3,3)
    ColumnMajor,
}

impl ParameterOrdering {
    pub fn get_coordinates(&self, block_size: usize) -> Vec<(usize, usize)> {
        match self {
            ParameterOrdering::RowMajor => (0..block_size)
                .flat_map(|row| (0..=row).map(move |col| (row, col)))
                .collect(),
            ParameterOrdering::ColumnMajor => (0..block_size)
                .flat_map(|col| (col..block_size).map(move |row| (row, col)))
                .collect(),
        }
    }

    /// Returns (storage_idx, row, col) for each coordinate in this ordering.
    ///
    /// `storage_idx` is the row-major index into the block parameter array, so
    /// callers can emit names in any ordering while still selecting the correct
    /// stored parameter.
    pub fn get_indexed_coordinates(&self, block_size: usize) -> Vec<(usize, usize, usize)> {
        let storage_coords = ParameterOrdering::RowMajor.get_coordinates(block_size);

        self.get_coordinates(block_size)
            .into_iter()
            .map(|(row, col)| {
                let storage_idx = storage_coords
                    .iter()
                    .position(|&(storage_row, storage_col)| {
                        storage_row == row && storage_col == col
                    })
                    .expect("requested coordinate must exist in row-major storage");
                (storage_idx, row, col)
            })
            .collect()
    }
}

impl Model {
    /// Iterate over OMEGA parameters in specified order.
    /// `block_fixed` is true when the block containing this parameter is fixed.
    pub fn get_omega_parameters(
        &self,
        ordering: ParameterOrdering,
    ) -> anyhow::Result<Vec<OmegaSigmaEntry>> {
        get_block_parameter_names(&self.omega_blocks, ordering, OMEGA, ETA)
    }

    /// Iterate over SIGMA parameters in specified order.
    /// `block_fixed` is true when the block containing this parameter is fixed.
    pub fn get_sigma_parameters(
        &self,
        ordering: ParameterOrdering,
    ) -> anyhow::Result<Vec<OmegaSigmaEntry>> {
        get_block_parameter_names(&self.sigma_blocks, ordering, SIGMA, EPS)
    }

    /// Validate that all parameter comments parse correctly for the given type.
    /// Returns the raw strings of comments that failed to parse.
    pub fn validate_comments(&self, comment_type: CommentType) -> Vec<String> {
        let mut failed = Vec::new();

        for theta in &self.thetas {
            if let Some(c) = theta.comment.as_deref()
                && parse_theta_param(c, comment_type).is_none()
            {
                failed.push(c.to_string());
            }
        }

        for block in &self.omega_blocks {
            for p in &block.parameters {
                if let Some(c) = p.comment.as_deref()
                    && parse_omega_param(c, comment_type).is_none()
                {
                    failed.push(c.to_string());
                }
            }
        }

        for block in &self.sigma_blocks {
            for p in &block.parameters {
                if let Some(c) = p.comment.as_deref()
                    && parse_sigma_param(c, comment_type).is_none()
                {
                    failed.push(c.to_string());
                }
            }
        }

        failed
    }

    /// Per-parameter info keyed by NONMEM column name, with OMEGA/SIGMA in row-major
    /// order and BLOCK SAME expanded.
    ///
    /// For each theta: `"THETA1" -> label "TVCL"` (if the comment parses)
    /// For each omega/sigma: `"OMEGA(1,1)" -> label "OM1 (TVCL)"` (if the comment parses)
    ///
    /// Every key is present regardless of `comment_type`; comments only fill in the
    /// label and transform. Returns `None` under `$MSFI`: the estimates come from the
    /// MSF file, so the control stream does not say which parameters exist.
    pub fn parameters(
        &self,
        comment_type: Option<CommentType>,
    ) -> anyhow::Result<Option<BTreeMap<String, ParameterInfo>>> {
        if self.msfi.is_some() {
            return Ok(None);
        }

        let mut parameters = BTreeMap::new();

        for (i, param) in self.thetas.iter().enumerate() {
            let parsed = comment_type
                .zip(param.comment.as_deref())
                .and_then(|(ct, c)| parse_theta_param(c, ct))
                .map(|p| ParameterInfo {
                    label: p.name(),
                    transform: p.transform(),
                })
                .unwrap_or_default();
            parameters.insert(format!("THETA{}", i + 1), parsed);
        }

        for entry in self.get_omega_parameters(ParameterOrdering::RowMajor)? {
            let parsed = comment_type
                .zip(entry.parameter.comment.as_deref())
                .and_then(|(ct, c)| parse_omega_param(c, ct))
                .map(|p| ParameterInfo {
                    label: p.name(),
                    transform: p.transform(),
                })
                .unwrap_or_default();
            parameters.insert(entry.param_name, parsed);
        }

        for entry in self.get_sigma_parameters(ParameterOrdering::RowMajor)? {
            let parsed = comment_type
                .zip(entry.parameter.comment.as_deref())
                .and_then(|(ct, c)| parse_sigma_param(c, ct))
                .map(|p| ParameterInfo {
                    label: p.name(),
                    transform: p.transform(),
                })
                .unwrap_or_default();
            parameters.insert(entry.param_name, parsed);
        }

        Ok(Some(parameters))
    }
}

fn get_block_parameter_names(
    blocks: &[OmegaSigmaBlock],
    ordering: ParameterOrdering,
    param_prefix: &str,
    raneff_prefix: &str,
) -> anyhow::Result<Vec<OmegaSigmaEntry>> {
    let mut results = Vec::new();
    let mut base_counter = 1;

    for (block_index, block) in blocks.iter().enumerate() {
        match &block.structure {
            BlockStructure::Diagonal => {
                for (param_idx, param) in block.parameters.iter().enumerate() {
                    let num = base_counter + param_idx;
                    results.push(OmegaSigmaEntry {
                        param_name: format!("{param_prefix}({num},{num})"),
                        raneff_label: format!("{raneff_prefix}{num}"),
                        parameter: param.clone(),
                        block_fixed: block.fixed,
                    });
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
                    let raneff_label = if row == col {
                        format!("{raneff_prefix}{param_row}")
                    } else {
                        format!("{raneff_prefix}{param_col}:{raneff_prefix}{param_row}")
                    };
                    results.push(OmegaSigmaEntry {
                        param_name: format!("{param_prefix}({param_row},{param_col})"),
                        raneff_label,
                        parameter: param.clone(),
                        block_fixed: block.fixed,
                    });
                }
                base_counter += size;
            }
            BlockStructure::BlockSame { size, repeats } => {
                // Search backwards for the most recent Block with matching size.
                // The lowerer validates that the chain is unbroken (no intervening
                // mismatched blocks), so this always finds the correct reference.
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
                    bail!("BlockSame {{size: {size}}} has no preceding Block {{size: {size}}}")
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
                        let raneff_label = if row == col {
                            format!("{raneff_prefix}{param_row}")
                        } else {
                            format!("{raneff_prefix}{param_col}:{raneff_prefix}{param_row}")
                        };
                        results.push(OmegaSigmaEntry {
                            param_name: format!("{param_prefix}({param_row},{param_col})"),
                            raneff_label,
                            parameter: param.clone(),
                            block_fixed: block.fixed,
                        });
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
    use super::{ParameterInfo, ParameterOrdering};
    use std::collections::BTreeMap;

    use crate::comments::{
        CommentType, Transform, parse_omega_param, parse_sigma_param, parse_theta_param,
    };
    use crate::model::Model;

    fn parse_model(input: &str) -> Model {
        Model::inner_parse(input).unwrap()
    }

    fn load_model(name: &str) -> Model {
        let path = format!("{}/test_data/{name}", env!("CARGO_MANIFEST_DIR"));
        let input = fs_err::read_to_string(&path).unwrap();
        parse_model(&input)
    }

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
            .map(|e| (e.param_name, e.raneff_label))
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
            .map(|e| (e.param_name, e.raneff_label))
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
            .map(|e| (e.param_name, e.raneff_label))
            .collect();
        insta::assert_debug_snapshot!(pairs);
    }

    fn labels(
        params: &Option<BTreeMap<String, ParameterInfo>>,
    ) -> BTreeMap<String, Option<String>> {
        params
            .as_ref()
            .unwrap()
            .iter()
            .map(|(k, v)| (k.clone(), v.label.clone()))
            .collect()
    }

    #[test]
    fn parameter_names_type1_comments() {
        let model = load_model("comments/type1.mod");
        let names = labels(&model.parameters(Some(CommentType::Type1)).unwrap());
        insta::assert_debug_snapshot!(names);
    }

    #[test]
    fn parameter_names_no_type1_comments() {
        let model =
            parse_model("$PROBLEM none\n$THETA 1\n$THETA 2 ; free text\n$OMEGA 0.1\n$SIGMA 0.1\n");
        let params = model.parameters(Some(CommentType::Type1)).unwrap();
        let names = labels(&params);
        assert_eq!(names.len(), 4);
        for (key, value) in &names {
            assert!(value.is_none(), "expected None for {key}, got {value:?}");
        }
        assert!(
            params
                .unwrap()
                .values()
                .all(|info| info.transform.is_none())
        );
    }

    #[test]
    fn parameter_names_type2_omega_comments_include_theta_refs() {
        let input = r#"
$PROBLEM type2 omega names
$THETA
(0, 1) ; CL
(0, 1) ; V
$OMEGA
0.1 ; IIV CL ;exp
0.2 ; IIV V ;exp
"#;

        let model = parse_model(input);
        let names = labels(&model.parameters(Some(CommentType::Type2)).unwrap());

        assert_eq!(names.get("OMEGA(1,1)"), Some(&Some("IIV (CL)".to_string())));
        assert_eq!(names.get("OMEGA(2,2)"), Some(&Some("IIV (V)".to_string())));
    }

    const TAGGED: &str = "$PROBLEM tagged\n\
$THETA (0, 1) ; TVCL (L/h) :LOG\n\
$THETA (0, 1) ; V :LOG\n\
$THETA (0, 1) ; F1 (frac) :logit\n\
$THETA (0, 1) ; WT cov\n\
$THETA (0, 1) ; TVKA (1/h)\n\
$THETA (0, 1) ; RES ERR :stdev\n\
$THETA (0, 1) ; BAD (x) :LGO\n\
$OMEGA 0.1 ; OM1 TVCL :LOG\n\
$OMEGA 0.1 ; OM2 V :prop\n\
$SIGMA 0.1 ; SIG1 :add\n\
$SIGMA 0.1 ; SIG2\n";

    #[test]
    fn type1_tags_become_transforms() {
        let model = parse_model(TAGGED);
        let params = model.parameters(Some(CommentType::Type1)).unwrap().unwrap();
        let transform = |key: &str| params[key].transform.clone();

        assert_eq!(transform("THETA1"), Some(Transform::LogNormal));
        assert_eq!(params["THETA1"].label.as_deref(), Some("TVCL"));
        assert_eq!(transform("THETA2"), Some(Transform::LogNormal));
        assert_eq!(params["THETA2"].label.as_deref(), Some("V"));
        assert_eq!(transform("THETA3"), Some(Transform::Logit));
        assert_eq!(transform("THETA4"), None);
        assert_eq!(transform("THETA5"), None);
        assert_eq!(transform("THETA6"), None);
        assert_eq!(transform("THETA7"), None);
        assert_eq!(transform("OMEGA(1,1)"), Some(Transform::LogNormal));
        assert_eq!(transform("OMEGA(2,2)"), Some(Transform::Proportional));
        assert_eq!(transform("SIGMA(1,1)"), Some(Transform::AddErr));
        assert_eq!(transform("SIGMA(2,2)"), None);
    }

    #[test]
    fn type2_tags_become_transforms() {
        let model = parse_model(
            "$PROBLEM t2\n$THETA (0, 1) ; CL ;log\n$THETA (0, 1) ; V\n$OMEGA 0.1 ; IIV CL ;exp\n$SIGMA 0.1 ; PROP ;prop\n",
        );
        let params = model.parameters(Some(CommentType::Type2)).unwrap().unwrap();
        assert_eq!(params["THETA1"].transform, Some(Transform::LogNormal));
        assert_eq!(params["THETA2"].transform, None);
        assert_eq!(params["OMEGA(1,1)"].transform, Some(Transform::LogNormal));
        assert_eq!(
            params["SIGMA(1,1)"].transform,
            Some(Transform::Proportional)
        );
    }

    #[test]
    fn keys_do_not_depend_on_comment_type() {
        let model = parse_model(TAGGED);
        let keys = |ct: Option<CommentType>| -> Vec<String> {
            model.parameters(ct).unwrap().unwrap().into_keys().collect()
        };
        let none = keys(None);
        assert_eq!(none.len(), 11);
        assert_eq!(keys(Some(CommentType::Type1)), none);
        assert_eq!(keys(Some(CommentType::Type2)), none);

        let params = model.parameters(None).unwrap().unwrap();
        assert!(params.values().all(|i| *i == ParameterInfo::default()));
    }

    #[test]
    fn msfi_has_no_parameter_map() {
        let model = parse_model(
            "$PROBLEM msfi\n$INPUT ID DV\n$DATA data.csv\n$MSFI run001.msf\n$PRED\nY = THETA(1) + ETA(1)\n$EST METHOD=1\n",
        );
        assert_eq!(model.parameters(Some(CommentType::Type1)).unwrap(), None);
    }

    #[test]
    fn type1_parsing_does_not_match_non_type1_comments() {
        let model = load_model("everything.mod");
        let mut comments = 0;
        for c in model.thetas.iter().filter_map(|t| t.comment.as_deref()) {
            comments += 1;
            assert!(parse_theta_param(c, CommentType::Type1).is_none(), "{c:?}");
        }
        for c in model
            .omega_blocks
            .iter()
            .flat_map(|b| &b.parameters)
            .filter_map(|p| p.comment.as_deref())
        {
            comments += 1;
            assert!(parse_omega_param(c, CommentType::Type1).is_none(), "{c:?}");
        }
        for c in model
            .sigma_blocks
            .iter()
            .flat_map(|b| &b.parameters)
            .filter_map(|p| p.comment.as_deref())
        {
            comments += 1;
            assert!(parse_sigma_param(c, CommentType::Type1).is_none(), "{c:?}");
        }
        assert!(
            comments > 0,
            "everything.mod should have parameter comments"
        );
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

        let names: Vec<_> = omega_names.into_iter().map(|e| e.param_name).collect();

        assert_eq!(
            names,
            vec![
                "OMEGA(1,1)".to_string(),
                "OMEGA(2,2)".to_string(),
                "OMEGA(3,3)".to_string(),
            ]
        );
    }

    #[test]
    fn block_same_without_parens_inherits_size() {
        let input = r#"
$PROBLEM same block indexing
$OMEGA BLOCK(2)
0.1
0.01 0.1
$OMEGA BLOCK SAME
"#;

        let model = parse_model(input);
        let omega_names = model
            .get_omega_parameters(ParameterOrdering::RowMajor)
            .unwrap();

        let names: Vec<_> = omega_names.into_iter().map(|e| e.param_name).collect();

        assert_eq!(
            names,
            vec![
                "OMEGA(1,1)".to_string(),
                "OMEGA(2,1)".to_string(),
                "OMEGA(2,2)".to_string(),
                "OMEGA(3,3)".to_string(),
                "OMEGA(4,3)".to_string(),
                "OMEGA(4,4)".to_string(),
            ]
        );
    }

    #[test]
    fn can_map_column_major_coordinates_to_row_major_storage() {
        let indexed = ParameterOrdering::ColumnMajor.get_indexed_coordinates(3);

        assert_eq!(
            indexed,
            vec![
                (0, 0, 0),
                (1, 1, 0),
                (3, 2, 0),
                (2, 1, 1),
                (4, 2, 1),
                (5, 2, 2),
            ]
        );
    }
}
