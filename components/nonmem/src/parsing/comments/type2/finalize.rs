use std::collections::HashMap;

use super::super::{
    ParamName, ParamPrefix, ParsedOmegaComment, ParsedSigmaComment, ParsedThetaComment,
};
use super::{Type2Omega, Type2ThetaSigma};
use crate::parsing::model::{BlockStructure, Model, Parameter, ParameterBlock, block_positions};

use super::parse::UnresolvedOmega;

struct ResolvedComments {
    thetas: Vec<Option<Type2ThetaSigma>>,
    omegas: Vec<Vec<Option<Type2Omega>>>,
    sigmas: Vec<Vec<Option<Type2ThetaSigma>>>,
}

struct ResolvedOmegaResult {
    omega: Option<Type2Omega>,
    errors: Vec<String>,
}

/// Theta reference entry for resolving omega theta_refs.
#[derive(Debug, Clone)]
struct ThetaReference {
    // If NAME= is used in control stream
    // this is the final name, otherwise it
    // is the comment name
    final_name: String,
    // both NAME= and comment names for lookup in
    // omega theta reference resolution.
    candidates: Vec<String>,
}

impl ThetaReference {
    fn new(final_name: String, alias: Option<&str>) -> Self {
        let mut candidates = vec![final_name.clone()];
        if let Some(alias) = alias
            && !candidates
                .iter()
                .any(|existing| existing.eq_ignore_ascii_case(alias))
        {
            candidates.push(alias.to_string());
        }
        Self {
            final_name,
            candidates,
        }
    }

    fn matches_exact(&self, raw: &str) -> bool {
        self.candidates.iter().any(|c| c.eq_ignore_ascii_case(raw))
    }

    fn resolve(raw: &str, refs: &[Self]) -> Option<String> {
        for theta in refs {
            if theta.matches_exact(raw) {
                return Some(theta.final_name.clone());
            }
        }
        None
    }
}

/// Resolve references, validate inferred metadata, and apply parsed comments to the model.
pub fn finalize_and_apply(
    model: &mut Model,
    thetas: Vec<Option<Type2ThetaSigma>>,
    omegas: Vec<Vec<Option<UnresolvedOmega>>>,
    sigmas: Vec<Vec<Option<Type2ThetaSigma>>>,
    errors: &mut Vec<String>,
) {
    let resolved = build_resolved_comments(model, thetas, omegas, sigmas, errors);
    validate_prefix_positions(&resolved, model, errors);
    apply_resolved_comments(model, resolved);
}

fn build_resolved_comments(
    model: &Model,
    mut thetas: Vec<Option<Type2ThetaSigma>>,
    omegas: Vec<Vec<Option<UnresolvedOmega>>>,
    mut sigmas: Vec<Vec<Option<Type2ThetaSigma>>>,
    errors: &mut Vec<String>,
) -> ResolvedComments {
    apply_explicit_theta_names(&mut thetas, model);
    let theta_refs = build_theta_references(&thetas, model);
    let mut omegas = resolve_omega_blocks(&omegas, model, &theta_refs, errors);
    apply_explicit_omega_names(&mut omegas, model);
    apply_explicit_sigma_names(&mut sigmas, model);
    validate_duplicate_thetas(&thetas, &model.theta_parameters, errors);
    validate_duplicate_omegas(&omegas, errors);
    validate_duplicate_sigmas(&sigmas, &model.sigma_blocks, errors);

    ResolvedComments {
        thetas,
        omegas,
        sigmas,
    }
}

fn apply_explicit_theta_names(thetas: &mut [Option<Type2ThetaSigma>], model: &Model) {
    for (parsed, param) in thetas.iter_mut().zip(model.theta_parameters.iter()) {
        if let (Some(explicit_name), Some(theta)) = (param.name.clone(), parsed.as_mut()) {
            theta.name = explicit_name;
        }
    }
}

fn apply_explicit_omega_names(omegas: &mut [Vec<Option<Type2Omega>>], model: &Model) {
    for (block_omegas, block) in omegas.iter_mut().zip(model.omega_blocks.iter()) {
        for (parsed, param) in block_omegas.iter_mut().zip(block.parameters.iter()) {
            if let (Some(explicit_name), Some(omega)) = (param.name.clone(), parsed.as_mut()) {
                omega.name = explicit_name;
            }
        }
    }
}

fn apply_explicit_sigma_names(sigmas: &mut [Vec<Option<Type2ThetaSigma>>], model: &Model) {
    for (block_sigmas, block) in sigmas.iter_mut().zip(model.sigma_blocks.iter()) {
        for (parsed, param) in block_sigmas.iter_mut().zip(block.parameters.iter()) {
            if let (Some(explicit_name), Some(sigma)) = (param.name.clone(), parsed.as_mut()) {
                sigma.name = explicit_name;
            }
        }
    }
}

fn build_theta_references(
    thetas: &[Option<Type2ThetaSigma>],
    model: &Model,
) -> Vec<ThetaReference> {
    model
        .theta_parameters
        .iter()
        .zip(thetas.iter())
        .filter_map(|(param, parsed)| {
            let final_name = param
                .name
                .clone()
                .or_else(|| parsed.as_ref().map(|p| p.name.clone()))?;
            let alias = parsed.as_ref().map(|p| p.name.as_str());
            Some(ThetaReference::new(final_name, alias))
        })
        .collect()
}

fn resolve_omega_blocks(
    unresolved_blocks: &[Vec<Option<UnresolvedOmega>>],
    model: &Model,
    theta_refs: &[ThetaReference],
    errors: &mut Vec<String>,
) -> Vec<Vec<Option<Type2Omega>>> {
    let omega_positions = block_positions(&model.omega_blocks);
    let mut resolved_omegas: Vec<Vec<Option<Type2Omega>>> = Vec::new();
    let mut pos_offset = 0;

    for (block_idx, block_unresolved) in unresolved_blocks.iter().enumerate() {
        let block = match model.omega_blocks.get(block_idx) {
            Some(b) => b,
            None => {
                resolved_omegas.push(vec![None; block_unresolved.len()]);
                continue;
            }
        };

        let block_len = block.parameter_count();
        let positions = &omega_positions[pos_offset..pos_offset + block_len];
        // skip duplicate off diagonal parsed comments if they match the diagonal
        // in BLOCK(N) with multiple parameters on one line the comment is assumed to refer to
        // diagonal entry.
        let skip = duplicate_row_skip(&block.structure, &block.parameters);

        let resolved_block_omegas: Vec<Option<Type2Omega>> = block_unresolved
            .iter()
            .enumerate()
            .map(|(param_idx, omega_opt)| {
                if skip[param_idx] {
                    None
                } else if let Some(omega) = omega_opt.as_ref() {
                    let is_diagonal = positions
                        .get(param_idx)
                        .map(|(r, c)| r == c)
                        .unwrap_or(true);
                    let result = resolve_block_omega_parameter(omega, is_diagonal, theta_refs);
                    errors.extend(result.errors);
                    result.omega
                } else {
                    None
                }
            })
            .collect();

        if matches!(block.structure, BlockStructure::Block { .. }) {
            errors.extend(validate_structural_off_diagonal_associations(
                &resolved_block_omegas,
                positions,
            ));
        }

        resolved_omegas.push(resolved_block_omegas);
        pos_offset += block.parameter_count();
    }

    resolved_omegas
}

/// Resolve one OMEGA parameter comment within a block position.
///
/// This validates the expected number of theta refs for diagonal vs
/// off-diagonal positions, resolves refs against known theta names, and in
/// non-strict mode preserves unresolved raw refs so the rest of the parsed
/// omega metadata is still available.
fn resolve_block_omega_parameter(
    omega: &UnresolvedOmega,
    is_diagonal: bool,
    theta_refs: &[ThetaReference],
) -> ResolvedOmegaResult {
    let mut errors = Vec::new();
    let (expected_refs, position) = if is_diagonal {
        (1, "diagonal")
    } else {
        (2, "off-diagonal")
    };

    if omega.raw_theta_refs.len() != expected_refs {
        errors.push(format!(
            "OMEGA comment '{}' has {} theta ref(s) but {} position expects {}",
            omega.name,
            omega.raw_theta_refs.len(),
            position,
            expected_refs,
        ));
    }

    // In non-strict mode, preserve raw theta refs that fail lookup so
    // the omega comment's other metadata is still available. Errors are
    // always recorded and may later become fatal via error_on_invalid.
    let mut associated_theta_values = Vec::with_capacity(omega.raw_theta_refs.len());
    let mut unknown = Vec::new();
    for r in &omega.raw_theta_refs {
        match ThetaReference::resolve(r, theta_refs) {
            Some(name) => associated_theta_values.push(name),
            None => {
                unknown.push(r.as_str());
                associated_theta_values.push(r.clone());
            }
        }
    }

    if !unknown.is_empty() {
        let theta_names: Vec<&str> = theta_refs.iter().map(|t| t.final_name.as_str()).collect();
        errors.push(format!(
            "OMEGA comment '{}' references unknown theta(s) [{}], known thetas are [{}]",
            omega.raw_comment.trim(),
            unknown.join(", "),
            theta_names.join(", "),
        ));
    }

    ResolvedOmegaResult {
        omega: Some(Type2Omega {
            prefix: omega.prefix.clone(),
            name: omega.name.clone(),
            associated_theta: Some(associated_theta_values),
            parameterization: omega.parameterization,
        }),
        errors,
    }
}

fn validate_prefix_positions(resolved: &ResolvedComments, model: &Model, errors: &mut Vec<String>) {
    validate_theta_prefixes(&resolved.thetas, errors);
    validate_block_prefixes(&resolved.omegas, &model.omega_blocks, "OMEGA", errors);
    validate_block_prefixes(&resolved.sigmas, &model.sigma_blocks, "SIGMA", errors);
}

fn apply_resolved_comments(model: &mut Model, resolved: ResolvedComments) {
    let ResolvedComments {
        thetas,
        omegas,
        sigmas,
    } = resolved;

    for (parsed, param) in thetas.into_iter().zip(model.theta_parameters.iter_mut()) {
        param.parsed_comment = parsed.map(ParsedThetaComment::Type2);
    }

    for (block_omegas, block) in omegas.into_iter().zip(model.omega_blocks.iter_mut()) {
        for (parsed, param) in block_omegas.into_iter().zip(block.parameters.iter_mut()) {
            param.parsed_comment = parsed.map(ParsedOmegaComment::Type2);
        }
    }

    for (block_sigmas, block) in sigmas.into_iter().zip(model.sigma_blocks.iter_mut()) {
        for (parsed, param) in block_sigmas.into_iter().zip(block.parameters.iter_mut()) {
            param.parsed_comment = parsed.map(ParsedSigmaComment::Type2);
        }
    }
}
/// Validate that resolved off-diagonal OMEGA associations agree with the
/// diagonal entries implied by block structure.
///
/// It reports inconsistencies between an off-diagonal comment and the
/// theta names implied by the corresponding diagonal OMEGA entries.
fn validate_structural_off_diagonal_associations(
    omegas: &[Option<Type2Omega>],
    positions: &[(usize, usize)],
) -> Vec<String> {
    let mut errors = Vec::new();

    for (param_idx, &(row, col)) in positions.iter().enumerate() {
        if row == col || param_idx >= omegas.len() {
            continue;
        }

        if let Some(error) = validate_off_diagonal_position(omegas, positions, param_idx, row, col)
        {
            errors.push(error);
        }
    }

    errors
}

fn validate_off_diagonal_position(
    omegas: &[Option<Type2Omega>],
    positions: &[(usize, usize)],
    param_idx: usize,
    row: usize,
    col: usize,
) -> Option<String> {
    let structural_assoc = structural_off_diagonal_assoc(omegas, positions, row, col)?;
    let omega = omegas.get(param_idx)?.as_ref()?;
    omega.associated_theta.as_ref()?;
    validate_off_diagonal_association(omega, row, col, &structural_assoc)
}

fn validate_off_diagonal_association(
    omega: &Type2Omega,
    row: usize,
    col: usize,
    structural_assoc: &[String],
) -> Option<String> {
    let explicit_assoc = omega.associated_theta.as_ref()?;
    // off diagonal associated thetas can be in any order
    // Corr CL,V is same as Corr V,CL
    // can consider making this stricter
    let matches_assoc = explicit_assoc == structural_assoc
        || explicit_assoc.iter().rev().eq(structural_assoc.iter());
    (!matches_assoc).then(|| {
        format!(
            "OMEGA({row},{col}) comment references theta(s) [{}] but block structure implies [{}]",
            explicit_assoc.join(", "),
            structural_assoc.join(", "),
        )
    })
}

fn structural_off_diagonal_assoc(
    omegas: &[Option<Type2Omega>],
    positions: &[(usize, usize)],
    row: usize,
    col: usize,
) -> Option<Vec<String>> {
    let row_theta = diagonal_associated_theta(omegas, positions, row)?;
    let col_theta = diagonal_associated_theta(omegas, positions, col)?;
    Some(vec![col_theta, row_theta])
}

fn diagonal_associated_theta(
    omegas: &[Option<Type2Omega>],
    positions: &[(usize, usize)],
    row: usize,
) -> Option<String> {
    let diag_idx = positions.iter().position(|&(r, c)| r == row && c == row)?;
    let omega = omegas.get(diag_idx)?.as_ref()?;
    omega
        .associated_theta
        .as_ref()
        .and_then(|a| a.first().cloned())
}

fn validate_duplicate_thetas(
    thetas: &[Option<Type2ThetaSigma>],
    params: &[Parameter<ParsedThetaComment>],
    errors: &mut Vec<String>,
) {
    use std::collections::hash_map::Entry;

    let mut seen = HashMap::<String, Option<String>>::new();

    for idx in 0..thetas.len() {
        let Some(name) = params[idx]
            .name
            .clone()
            .or_else(|| thetas[idx].as_ref().map(|p| p.name.clone()))
        else {
            continue;
        };

        let key = name.to_ascii_lowercase();
        match seen.entry(key) {
            Entry::Vacant(e) => {
                e.insert(Some(name));
            }
            Entry::Occupied(mut e) => {
                if let Some(first_name) = e.get_mut().take() {
                    errors.push(format!("Duplicate theta name: {first_name}"));
                }
                errors.push(format!("Duplicate theta name: {name}"));
            }
        }
    }
}

fn validate_duplicate_omegas(omegas: &[Vec<Option<Type2Omega>>], errors: &mut Vec<String>) {
    let mut counts = HashMap::<String, usize>::new();
    for block in omegas.iter() {
        for omega in block.iter().flatten() {
            *counts.entry(duplicate_key(omega)).or_default() += 1;
        }
    }

    for block in omegas.iter() {
        for omega in block.iter().flatten() {
            let key = duplicate_key(omega);
            if counts.get(&key).copied().unwrap_or(0) > 1 {
                let assoc = omega
                    .associated_theta
                    .as_ref()
                    .map(|xs| xs.join("-"))
                    .unwrap_or_default();
                errors.push(format!(
                    "Duplicate OMEGA comment identity: name='{}', associated_theta='{}'",
                    omega.name, assoc
                ));
            }
        }
    }
}

fn validate_duplicate_sigmas(
    sigmas: &[Vec<Option<Type2ThetaSigma>>],
    blocks: &[ParameterBlock<ParsedSigmaComment>],
    errors: &mut Vec<String>,
) {
    use std::collections::hash_map::Entry;

    let mut seen = HashMap::<String, Option<String>>::new();

    for (block_sigmas, block) in sigmas.iter().zip(blocks.iter()) {
        for (sigma_opt, param) in block_sigmas.iter().zip(block.parameters.iter()) {
            let Some(name) = param
                .name
                .clone()
                .or_else(|| sigma_opt.as_ref().map(|p| p.name.clone()))
            else {
                continue;
            };

            let key = name.to_ascii_lowercase();
            match seen.entry(key) {
                Entry::Vacant(e) => {
                    e.insert(Some(name));
                }
                Entry::Occupied(mut e) => {
                    if let Some(first_name) = e.get_mut().take() {
                        errors.push(format!("Duplicate sigma name: {first_name}"));
                    }
                    errors.push(format!("Duplicate sigma name: {name}"));
                }
            }
        }
    }
}

fn duplicate_key(omega: &Type2Omega) -> String {
    let assoc = omega
        .associated_theta
        .as_ref()
        .map(|xs| xs.join("-"))
        .unwrap_or_default();
    format!(
        "{}|{}",
        omega.name.to_ascii_lowercase(),
        assoc.to_ascii_lowercase()
    )
}

/// Skip off-diagonal elements whose raw comment string duplicates a diagonal element's.
///
/// When multiple parameters share a source line, the parser assigns the same comment
/// to all of them. This detects and skips those duplicates.
fn duplicate_row_skip<T: ParamName>(
    structure: &BlockStructure,
    parameters: &[Parameter<T>],
) -> Vec<bool> {
    let mut skip = vec![false; parameters.len()];
    let BlockStructure::Block { size } = *structure else {
        return skip;
    };

    let comment_at = |idx: usize| -> Option<&str> {
        parameters[idx]
            .comment
            .as_deref()
            .map(str::trim)
            .filter(|c| !c.is_empty())
    };

    let mut row_start = 0usize;
    let mut previous_diag_comment: Option<&str> = None;
    for row_len in 1..=size {
        let row_end = row_start + row_len;
        if row_end > parameters.len() {
            break;
        }

        let diag_comment = comment_at(row_end - 1);

        for (skip_flag, idx) in skip[row_start..(row_end - 1)].iter_mut().zip(row_start..) {
            let current = comment_at(idx);
            if current.is_some() && (diag_comment == current || previous_diag_comment == current) {
                *skip_flag = true;
            }
        }

        previous_diag_comment = diag_comment;
        row_start = row_end;
    }
    skip
}

fn parse_theta_prefix_index(prefix: &str) -> Option<usize> {
    let stripped = prefix.trim_end_matches([':', '-', '.', ',']);
    let lower = stripped.to_ascii_lowercase();

    let digits = if lower.starts_with("theta") {
        let rest = &stripped["theta".len()..];
        rest.strip_prefix('(')
            .and_then(|s| s.strip_suffix(')'))
            .unwrap_or(rest)
    } else {
        stripped
    };

    digits.parse().ok()
}

#[derive(Debug, PartialEq, Eq)]
enum BlockPrefixPosition {
    Parsed((usize, usize)),
    AmbiguousDigits,
}

fn parse_block_prefix_position(prefix: &str, keyword: &str) -> Option<BlockPrefixPosition> {
    let stripped = prefix.trim_end_matches([':', '-', '.', ',']);
    let lower = stripped.to_ascii_lowercase();

    let digits_str = if lower.starts_with(keyword) {
        let rest = &stripped[keyword.len()..];
        rest.strip_prefix('(')
            .and_then(|s| s.strip_suffix(')'))
            .unwrap_or(rest)
    } else {
        stripped
    };

    if let Some((r, c)) = digits_str.split_once(',') {
        let row: usize = r.parse().ok()?;
        let col: usize = c.parse().ok()?;
        return Some(BlockPrefixPosition::Parsed((row, col)));
    }

    if digits_str.chars().all(|c| c.is_ascii_digit()) {
        match digits_str.len() {
            1 => {
                let n: usize = digits_str.parse().ok()?;
                return Some(BlockPrefixPosition::Parsed((n, n)));
            }
            2 => {
                let row: usize = digits_str[..1].parse().ok()?;
                let col: usize = digits_str[1..].parse().ok()?;
                return Some(BlockPrefixPosition::Parsed((row, col)));
            }
            _ => {
                return Some(BlockPrefixPosition::AmbiguousDigits);
            }
        }
    }

    None
}

fn validate_theta_prefixes(thetas: &[Option<Type2ThetaSigma>], errors: &mut Vec<String>) {
    for (idx, theta_opt) in thetas.iter().enumerate() {
        let Some(theta) = theta_opt else { continue };
        let Some(prefix) = theta.prefix.as_deref() else {
            continue;
        };
        let Some(claimed) = parse_theta_prefix_index(prefix) else {
            continue;
        };
        let actual = idx + 1;
        if claimed != actual {
            errors.push(format!(
                "Prefix mismatch: comment says THETA{claimed} but parameter is THETA{actual}"
            ));
        }
    }
}

fn validate_block_prefixes<T: ParamName, P: ParamPrefix>(
    blocks: &[Vec<Option<P>>],
    model_blocks: &[ParameterBlock<T>],
    prefix_keyword: &str,
    errors: &mut Vec<String>,
) {
    let positions = block_positions(model_blocks);
    let keyword_lower = prefix_keyword.to_ascii_lowercase();
    let mut pos_offset = 0;
    for (block_parsed, model_block) in blocks.iter().zip(model_blocks.iter()) {
        for (param_idx, parsed_opt) in block_parsed.iter().enumerate() {
            let Some(parsed) = parsed_opt else { continue };
            let Some(prefix) = parsed.prefix() else {
                continue;
            };
            let Some(actual_row_col) = positions.get(pos_offset + param_idx).copied() else {
                continue;
            };

            match parse_block_prefix_position(prefix, &keyword_lower) {
                Some(BlockPrefixPosition::Parsed(claimed))
                    if claimed != actual_row_col
                        && claimed != (actual_row_col.1, actual_row_col.0) =>
                {
                    errors.push(format!(
                        "Prefix mismatch: comment says {prefix_keyword}({},{}) but parameter is {prefix_keyword}({},{})",
                        claimed.0, claimed.1, actual_row_col.0, actual_row_col.1
                    ));
                }
                Some(BlockPrefixPosition::AmbiguousDigits) => {
                    errors.push(format!(
                        "Ambiguous {prefix_keyword} prefix '{prefix}'; use {prefix_keyword}(row,col) format"
                    ));
                }
                _ => {}
            }
        }
        pos_offset += model_block.parameter_count();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parsing::comments::Type2Omega;
    use crate::parsing::model::{BlockStructure, Parameter, ParameterBlock};

    #[test]
    fn parse_theta_prefix_index_cases() {
        assert_eq!(parse_theta_prefix_index("THETA1"), Some(1));
        assert_eq!(parse_theta_prefix_index("THETA8"), Some(8));
        assert_eq!(parse_theta_prefix_index("THETA(3)"), Some(3));
        assert_eq!(parse_theta_prefix_index("THETA12:"), Some(12));
        assert_eq!(parse_theta_prefix_index("5"), Some(5));
        assert_eq!(parse_theta_prefix_index("5:"), Some(5));
        assert_eq!(parse_theta_prefix_index("11"), Some(11));
    }

    #[test]
    fn parse_block_prefix_position_explicit() {
        assert_eq!(
            parse_block_prefix_position("OMEGA(1,1)", "omega"),
            Some(BlockPrefixPosition::Parsed((1, 1)))
        );
        assert_eq!(
            parse_block_prefix_position("OMEGA(2,1)", "omega"),
            Some(BlockPrefixPosition::Parsed((2, 1)))
        );
        assert_eq!(
            parse_block_prefix_position("SIGMA(3,3)", "sigma"),
            Some(BlockPrefixPosition::Parsed((3, 3)))
        );
        assert_eq!(
            parse_block_prefix_position("OMEGA(2,1):", "omega"),
            Some(BlockPrefixPosition::Parsed((2, 1)))
        );
    }

    #[test]
    fn parse_block_prefix_position_digits() {
        assert_eq!(
            parse_block_prefix_position("11", "omega"),
            Some(BlockPrefixPosition::Parsed((1, 1)))
        );
        assert_eq!(
            parse_block_prefix_position("21", "omega"),
            Some(BlockPrefixPosition::Parsed((2, 1)))
        );
        assert_eq!(
            parse_block_prefix_position("33", "omega"),
            Some(BlockPrefixPosition::Parsed((3, 3)))
        );
        assert_eq!(
            parse_block_prefix_position("22:", "omega"),
            Some(BlockPrefixPosition::Parsed((2, 2)))
        );

        assert_eq!(
            parse_block_prefix_position("1", "omega"),
            Some(BlockPrefixPosition::Parsed((1, 1)))
        );
        assert_eq!(
            parse_block_prefix_position("3", "omega"),
            Some(BlockPrefixPosition::Parsed((3, 3)))
        );

        assert_eq!(
            parse_block_prefix_position("121", "omega"),
            Some(BlockPrefixPosition::AmbiguousDigits)
        );
    }

    #[test]
    fn parse_block_prefix_position_labeled_digits() {
        assert_eq!(
            parse_block_prefix_position("OMEGA11", "omega"),
            Some(BlockPrefixPosition::Parsed((1, 1)))
        );
        assert_eq!(
            parse_block_prefix_position("OMEGA21", "omega"),
            Some(BlockPrefixPosition::Parsed((2, 1)))
        );
        assert_eq!(
            parse_block_prefix_position("SIGMA1", "sigma"),
            Some(BlockPrefixPosition::Parsed((1, 1)))
        );
        assert_eq!(
            parse_block_prefix_position("SIGMA112", "sigma"),
            Some(BlockPrefixPosition::AmbiguousDigits)
        );
    }

    #[test]
    fn validate_block_prefixes_reports_ambiguous_compact_digits() {
        let omegas = vec![vec![Some(Type2Omega {
            prefix: Some("OMEGA112".to_string()),
            name: "IIV_CL".to_string(),
            associated_theta: Some(vec!["CL".to_string()]),
            ..Default::default()
        })]];
        let blocks: Vec<ParameterBlock<Type2Omega>> = vec![ParameterBlock {
            structure: BlockStructure::Diagonal,
            parametrization: None,
            parameters: vec![Parameter {
                name: None,
                lower_bound: None,
                initial_value: 0.0,
                upper_bound: None,
                is_fixed: false,
                comment: None,
                parsed_comment: None,
            }],
        }];

        let mut errors = Vec::new();
        validate_block_prefixes(&omegas, &blocks, "OMEGA", &mut errors);

        assert_eq!(
            errors,
            vec!["Ambiguous OMEGA prefix 'OMEGA112'; use OMEGA(row,col) format".to_string()]
        );
    }

    #[test]
    fn resolve_block_omega_parameter_requires_exact_theta_match() {
        let omega = UnresolvedOmega {
            raw_comment: "IIV_CL CL".to_string(),
            prefix: None,
            name: "IIV_CL".to_string(),
            raw_theta_refs: vec!["CL".to_string()],
            parameterization: None,
        };
        let theta_refs = vec![
            ThetaReference::new("CL/F".to_string(), None),
            ThetaReference::new("CL/G".to_string(), None),
        ];

        let result = resolve_block_omega_parameter(&omega, true, &theta_refs);

        assert_eq!(
            result.errors,
            vec![
                "OMEGA comment 'IIV_CL CL' references unknown theta(s) [CL], known thetas are [CL/F, CL/G]".to_string()
            ]
        );
        assert_eq!(
            result
                .omega
                .as_ref()
                .and_then(|o| o.associated_theta.clone()),
            Some(vec!["CL".to_string()])
        );
    }

    #[test]
    fn validate_theta_prefixes_catches_mismatch() {
        let thetas = vec![
            Some(Type2ThetaSigma {
                prefix: Some("THETA1".to_string()),
                name: "CL".to_string(),
                ..Default::default()
            }),
            Some(Type2ThetaSigma {
                prefix: Some("THETA8".to_string()),
                name: "V".to_string(),
                ..Default::default()
            }),
            Some(Type2ThetaSigma {
                prefix: None,
                name: "KA".to_string(),
                ..Default::default()
            }),
        ];

        let mut errors = Vec::new();
        validate_theta_prefixes(&thetas, &mut errors);

        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("THETA8"));
        assert!(errors[0].contains("THETA2"));
    }

    #[test]
    fn validate_theta_prefixes_accepts_correct() {
        let thetas = vec![
            Some(Type2ThetaSigma {
                prefix: Some("1".to_string()),
                name: "CL".to_string(),
                ..Default::default()
            }),
            Some(Type2ThetaSigma {
                prefix: Some("THETA2".to_string()),
                name: "V".to_string(),
                ..Default::default()
            }),
        ];

        let mut errors = Vec::new();
        validate_theta_prefixes(&thetas, &mut errors);
        assert!(errors.is_empty());
    }

    #[test]
    fn validate_duplicate_thetas_reports_errors_without_mutating() {
        let thetas = vec![
            Some(Type2ThetaSigma {
                name: "CL".to_string(),
                ..Default::default()
            }),
            Some(Type2ThetaSigma {
                name: "CL".to_string(),
                ..Default::default()
            }),
        ];
        let params = vec![
            Parameter {
                name: None,
                lower_bound: None,
                initial_value: 0.0,
                upper_bound: None,
                is_fixed: false,
                comment: None,
                parsed_comment: None,
            },
            Parameter {
                name: None,
                lower_bound: None,
                initial_value: 0.0,
                upper_bound: None,
                is_fixed: false,
                comment: None,
                parsed_comment: None,
            },
        ];

        let mut errors = Vec::new();
        validate_duplicate_thetas(&thetas, &params, &mut errors);

        assert_eq!(
            errors,
            vec![
                "Duplicate theta name: CL".to_string(),
                "Duplicate theta name: CL".to_string(),
            ]
        );
        assert!(thetas[0].is_some());
        assert!(thetas[1].is_some());
    }

    #[test]
    fn validate_duplicate_omegas_reports_errors_without_mutating() {
        let omegas = vec![vec![
            Some(Type2Omega {
                name: "IIV".to_string(),
                associated_theta: Some(vec!["CL".to_string()]),
                ..Default::default()
            }),
            Some(Type2Omega {
                name: "IIV".to_string(),
                associated_theta: Some(vec!["CL".to_string()]),
                ..Default::default()
            }),
        ]];

        let mut errors = Vec::new();
        validate_duplicate_omegas(&omegas, &mut errors);

        assert_eq!(
            errors,
            vec![
                "Duplicate OMEGA comment identity: name='IIV', associated_theta='CL'".to_string(),
                "Duplicate OMEGA comment identity: name='IIV', associated_theta='CL'".to_string(),
            ]
        );
        assert!(omegas[0][0].is_some());
        assert!(omegas[0][1].is_some());
    }

    #[test]
    fn validate_duplicate_sigmas_reports_errors_without_mutating() {
        let sigmas = vec![vec![
            Some(Type2ThetaSigma {
                name: "PropErr".to_string(),
                ..Default::default()
            }),
            Some(Type2ThetaSigma {
                name: "PropErr".to_string(),
                ..Default::default()
            }),
        ]];
        let blocks = vec![ParameterBlock {
            structure: BlockStructure::Diagonal,
            parametrization: None,
            parameters: vec![
                Parameter {
                    name: None,
                    lower_bound: None,
                    initial_value: 0.0,
                    upper_bound: None,
                    is_fixed: false,
                    comment: None,
                    parsed_comment: None,
                },
                Parameter {
                    name: None,
                    lower_bound: None,
                    initial_value: 0.0,
                    upper_bound: None,
                    is_fixed: false,
                    comment: None,
                    parsed_comment: None,
                },
            ],
        }];

        let mut errors = Vec::new();
        validate_duplicate_sigmas(&sigmas, &blocks, &mut errors);

        assert_eq!(
            errors,
            vec![
                "Duplicate sigma name: PropErr".to_string(),
                "Duplicate sigma name: PropErr".to_string(),
            ]
        );
        assert!(sigmas[0][0].is_some());
        assert!(sigmas[0][1].is_some());
    }
}
