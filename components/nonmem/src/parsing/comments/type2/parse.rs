use super::super::CommentParameterization;
use super::Type2ThetaSigma;
use crate::parsing::model::Model;

/// Raw omega interpretation before theta reference resolution.
#[derive(Debug, Clone, PartialEq)]
pub struct UnresolvedOmega {
    pub raw_comment: String,
    pub prefix: Option<String>,
    pub name: String,
    // Not guaranteed to be a valid theta name
    pub raw_theta_refs: Vec<String>,
    pub parameterization: Option<CommentParameterization>,
}

pub struct ParsedComments {
    pub thetas: Vec<Option<Type2ThetaSigma>>,
    pub omegas: Vec<Vec<Option<UnresolvedOmega>>>,
    pub sigmas: Vec<Vec<Option<Type2ThetaSigma>>>,
    pub errors: Vec<String>,
}

/// Parse all parameter comments into unresolved type2 structs.
///
/// Each Vec mirrors the model's parameter ordering.
pub fn parse_all(model: &Model) -> ParsedComments {
    let mut errors = Vec::new();

    let thetas: Vec<Option<Type2ThetaSigma>> = model
        .theta_parameters
        .iter()
        .map(|p| {
            parse_comment(
                p.comment.as_deref(),
                |c| parse_theta_sigma(c, "THETA"),
                &mut errors,
            )
        })
        .collect();

    let omegas: Vec<Vec<Option<UnresolvedOmega>>> = model
        .omega_blocks
        .iter()
        .map(|block| {
            block
                .parameters
                .iter()
                .map(|p| parse_comment(p.comment.as_deref(), parse_omega, &mut errors))
                .collect()
        })
        .collect();

    let sigmas: Vec<Vec<Option<Type2ThetaSigma>>> = model
        .sigma_blocks
        .iter()
        .map(|block| {
            block
                .parameters
                .iter()
                .map(|p| {
                    parse_comment(
                        p.comment.as_deref(),
                        |c| parse_theta_sigma(c, "SIGMA"),
                        &mut errors,
                    )
                })
                .collect()
        })
        .collect();

    ParsedComments {
        thetas,
        omegas,
        sigmas,
        errors,
    }
}

fn parse_comment<T>(
    comment: Option<&str>,
    parser: impl FnOnce(&str) -> Result<Option<T>, String>,
    errors: &mut Vec<String>,
) -> Option<T> {
    let comment = comment?.trim();

    if comment.is_empty() {
        return None;
    }

    match parser(comment) {
        Ok(result) => result,
        Err(err) => {
            errors.push(err);
            None
        }
    }
}

fn parse_theta_sigma(comment: &str, label: &str) -> Result<Option<Type2ThetaSigma>, String> {
    let tokens: Vec<&str> = comment.split_whitespace().collect();
    if tokens.is_empty() {
        return Ok(None);
    }

    let (prefix, rest) = if let Some(pfx) = classify_prefix(tokens[0]) {
        (Some(pfx), &tokens[1..])
    } else {
        (None, tokens.as_slice())
    };

    let mut name_tokens = Vec::new();
    let mut unit = None;
    let mut transform_raw = None;

    for &token in rest {
        if token.starts_with(':') || token.starts_with(';') {
            transform_raw = Some(&token[1..]);
        } else if is_unit_token(token) {
            unit = Some(strip_unit_brackets(token).to_string());
        } else {
            name_tokens.push(token);
        }
    }

    let parameterization = parse_transform(transform_raw)?;

    let name = if name_tokens.len() == 1 {
        name_tokens[0].to_string()
    } else {
        return Err(format!(
            "Invalid type2 {label} comment: {comment}\n\
            prefix={}, unit={}, transform={:?}\n\
            Expected exactly one name token but found {}: {:?}",
            prefix.as_deref().unwrap_or("none"),
            unit.as_deref().unwrap_or("none"),
            parameterization,
            name_tokens.len(),
            name_tokens
        ));
    };

    Ok(Some(Type2ThetaSigma {
        prefix,
        name,
        unit,
        parameterization,
    }))
}

fn parse_omega(comment: &str) -> Result<Option<UnresolvedOmega>, String> {
    let tokens: Vec<&str> = comment.split_whitespace().collect();
    if tokens.is_empty() {
        return Ok(None);
    }

    let (prefix, rest) = if let Some(pfx) = classify_prefix(tokens[0]) {
        (Some(pfx), &tokens[1..])
    } else {
        (None, tokens.as_slice())
    };

    let mut positional = Vec::new();
    let mut transform_raw = None;

    for &token in rest {
        if token.starts_with(':') || token.starts_with(';') {
            transform_raw = Some(&token[1..]);
        } else {
            positional.push(token);
        }
    }

    let parameterization = parse_transform(transform_raw)?;

    let (name, raw_theta_refs) = match positional.as_slice() {
        [name, theta_ref] => {
            let refs = split_off_diagonal_theta_refs(theta_ref).ok_or_else(|| {
                format!(
                    "Invalid type2 OMEGA comment: {comment}\n\
                 prefix={}, transform={:?}\n\
                 Expected second positional token to contain valid THETA reference(s), \
                 but got: {:?}",
                    prefix.as_deref().unwrap_or("none"),
                    parameterization,
                    theta_ref
                )
            })?;
            (name.to_string(), refs)
        }
        _ => {
            return Err(format!(
                "Invalid type2 OMEGA comment: {comment}\n\
             prefix={}, transform={:?}\n\
             Expected exactly two positional tokens [name, theta_ref], \
             but found {}: {:?}",
                prefix.as_deref().unwrap_or("none"),
                parameterization,
                positional.len(),
                positional
            ));
        }
    };

    Ok(Some(UnresolvedOmega {
        raw_comment: comment.to_string(),
        prefix,
        name,
        raw_theta_refs,
        parameterization,
    }))
}

fn split_off_diagonal_theta_refs(raw: &str) -> Option<Vec<String>> {
    // Split on comma first if present, otherwise on hyphen
    for sep in [',', '-'] {
        if raw.contains(sep) {
            let parts: Vec<String> = raw
                .split(sep)
                .map(str::trim)
                .filter(|part| !part.is_empty())
                .map(ToString::to_string)
                .collect();
            if parts.len() == 2 {
                return Some(parts);
            }
            return None;
        }
    }
    Some(vec![raw.to_string()])
}

/// Classify the first token as a prefix if it matches known patterns.
///
/// Valid prefixes:
/// - Label: THETA|OMEGA|SIGMA + number component + optional trailing separator
/// - Bare number: digits + optional trailing separator
fn classify_prefix(token: &str) -> Option<String> {
    let stripped = token.trim_end_matches([':', '-', '.', ',']);
    if stripped.is_empty() {
        return None;
    }

    let lower = stripped.to_ascii_lowercase();
    for prefix in ["theta", "omega", "sigma"] {
        if lower.starts_with(prefix) {
            let remainder = &stripped[prefix.len()..];
            // labels like THETA/OMEGA/SIGMA cannot be used in
            // validation so they aren't kept
            if remainder.is_empty() {
                return None;
            }
            // THETA1, OMEGA11, SIGMA2 etc.
            if remainder.chars().all(|c| c.is_ascii_digit()) {
                return Some(stripped.to_string());
            }
            // OMEGA(1,1), THETA(1) etc.
            if remainder.starts_with('(') && remainder.ends_with(')') {
                let inner = &remainder[1..remainder.len() - 1];
                if !inner.is_empty() && inner.chars().all(|c| c.is_ascii_digit() || c == ',') {
                    return Some(stripped.to_string());
                }
            }
            return None;
        }
    }

    // Bare number prefix
    if stripped.chars().all(|c| c.is_ascii_digit()) {
        return Some(stripped.to_string());
    }

    None
}

fn is_unit_token(token: &str) -> bool {
    (token.starts_with('(') && token.ends_with(')'))
        || (token.starts_with('[') && token.ends_with(']'))
}

fn strip_unit_brackets(token: &str) -> &str {
    &token[1..token.len() - 1]
}

fn parse_transform(raw: Option<&str>) -> Result<Option<CommentParameterization>, String> {
    raw.map(|raw| {
        CommentParameterization::parse(raw)
            .ok_or_else(|| format!("Invalid parameterization: {raw}"))
    })
    .transpose()
}

#[cfg(test)]
mod tests {
    use super::*;
    use CommentParameterization as P;

    fn assert_theta_sigma_case(case: &ThetaSigmaCase) {
        let result = parse_theta_sigma(case.input, "THETA")
            .unwrap_or_else(|e| panic!("Failed for '{}': {e}", case.input))
            .unwrap_or_else(|| panic!("Got None for '{}'", case.input));
        assert_eq!(
            result.prefix.as_deref(),
            case.prefix,
            "prefix mismatch for '{}'",
            case.input
        );
        assert_eq!(result.name, case.name, "name mismatch for '{}'", case.input);
        assert_eq!(
            result.unit.as_deref(),
            case.unit,
            "unit mismatch for '{}'",
            case.input
        );
        assert_eq!(
            result.parameterization, case.param,
            "param mismatch for '{}'",
            case.input
        );
    }

    fn assert_omega_case(case: &OmegaCase) {
        let result = parse_omega(case.input)
            .unwrap_or_else(|e| panic!("Failed for '{}': {e}", case.input))
            .unwrap_or_else(|| panic!("Got None for '{}'", case.input));
        assert_eq!(
            result.prefix.as_deref(),
            case.prefix,
            "prefix mismatch for '{}'",
            case.input
        );
        assert_eq!(result.name, case.name, "name mismatch for '{}'", case.input);
        assert_eq!(
            result.raw_theta_refs, case.refs,
            "refs mismatch for '{}'",
            case.input
        );
        assert_eq!(
            result.parameterization, case.param,
            "param mismatch for '{}'",
            case.input
        );
    }

    #[test]
    fn classify_prefix_labels() {
        let cases = [
            ("THETA1", Some("THETA1")),
            ("THETA1:", Some("THETA1")),
            ("THETA(1)", Some("THETA(1)")),
            ("OMEGA(1,1)", Some("OMEGA(1,1)")),
            ("OMEGA11", Some("OMEGA11")),
            ("OMEGA(2,1)-", Some("OMEGA(2,1)")),
            ("SIGMA1:", Some("SIGMA1")),
            ("SIGMA(2,2).", Some("SIGMA(2,2)")),
            ("SIGMA1,", Some("SIGMA1")),
        ];

        for (input, expected) in cases {
            assert_eq!(
                classify_prefix(input),
                expected.map(str::to_string),
                "prefix classification mismatch for '{input}'"
            );
        }
    }

    #[test]
    fn classify_prefix_bare_numbers() {
        let cases = [
            ("1", Some("1")),
            ("1:", Some("1")),
            ("1-", Some("1")),
            ("1.", Some("1")),
            ("1,", Some("1")),
            ("11", Some("11")),
            ("22:", Some("22")),
        ];

        for (input, expected) in cases {
            assert_eq!(
                classify_prefix(input),
                expected.map(str::to_string),
                "prefix classification mismatch for '{input}'"
            );
        }
    }

    #[test]
    fn classify_prefix_rejects_invalid() {
        let cases = [
            "OM1", "SIG1", "SIG1:", "THETA", "OMEGA", "SIGMA", "CL", "IIV", "5FU",
        ];

        for input in cases {
            assert_eq!(
                classify_prefix(input),
                None,
                "expected no prefix for '{input}'"
            );
        }
    }

    struct ThetaSigmaCase {
        input: &'static str,
        prefix: Option<&'static str>,
        name: &'static str,
        unit: Option<&'static str>,
        param: Option<P>,
    }

    #[test]
    fn theta_sigma_ok_cases() {
        let cases = [
            ThetaSigmaCase {
                input: "CL",
                prefix: None,
                name: "CL",
                unit: None,
                param: None,
            },
            ThetaSigmaCase {
                input: "CL (L/day)",
                prefix: None,
                name: "CL",
                unit: Some("L/day"),
                param: None,
            },
            ThetaSigmaCase {
                input: "KA [1/(mg*h)]",
                prefix: None,
                name: "KA",
                unit: Some("1/(mg*h)"),
                param: None,
            },
            ThetaSigmaCase {
                input: "CL ;exp",
                prefix: None,
                name: "CL",
                unit: None,
                param: Some(P::LogNormal),
            },
            ThetaSigmaCase {
                input: "THETA1: CL (L/day) ;exp",
                prefix: Some("THETA1"),
                name: "CL",
                unit: Some("L/day"),
                param: Some(P::LogNormal),
            },
            ThetaSigmaCase {
                input: "1: CL (L/day) ;exp",
                prefix: Some("1"),
                name: "CL",
                unit: Some("L/day"),
                param: Some(P::LogNormal),
            },
            ThetaSigmaCase {
                input: "CL ;EXP (L/day)",
                prefix: None,
                name: "CL",
                unit: Some("L/day"),
                param: Some(P::LogNormal),
            },
            ThetaSigmaCase {
                input: "CL/F [L/h]",
                prefix: None,
                name: "CL/F",
                unit: Some("L/h"),
                param: None,
            },
            ThetaSigmaCase {
                input: "5FU (mg/L)",
                prefix: None,
                name: "5FU",
                unit: Some("mg/L"),
                param: None,
            },
            ThetaSigmaCase {
                input: "AddErr [ng/mL] :ADD",
                prefix: None,
                name: "AddErr",
                unit: Some("ng/mL"),
                param: Some(P::AddErr),
            },
            ThetaSigmaCase {
                input: "SIGMA2 AddErr :AddErr",
                prefix: Some("SIGMA2"),
                name: "AddErr",
                unit: None,
                param: Some(P::AddErr),
            },
        ];

        for case in &cases {
            assert_theta_sigma_case(case);
        }
    }

    #[test]
    fn theta_sigma_error_cases() {
        let error_inputs = [
            "CL :blahblah",
            "not a valid comment",
            "THETA1: CL WT () :EXP",
        ];
        for input in error_inputs {
            let result = parse_theta_sigma(input, "THETA");
            if let Err(e) = &result {
                println!("Input: '{input}' → Error: {e}");
            }
            assert!(result.is_err(), "Expected error for '{input}'");
        }
    }

    #[test]
    fn theta_sigma_none_cases() {
        let none_inputs = ["", "   "];
        for input in none_inputs {
            assert_eq!(
                parse_theta_sigma(input, "THETA").unwrap(),
                None,
                "Expected None for '{input}'"
            );
        }
    }

    struct OmegaCase {
        input: &'static str,
        prefix: Option<&'static str>,
        name: &'static str,
        refs: &'static [&'static str],
        param: Option<P>,
    }

    #[test]
    fn omega_ok_cases() {
        let cases = [
            OmegaCase {
                input: "IIV CL ;exp",
                prefix: None,
                name: "IIV",
                refs: &["CL"],
                param: Some(P::LogNormal),
            },
            OmegaCase {
                input: "11 IIV CL/F ;log",
                prefix: Some("11"),
                name: "IIV",
                refs: &["CL/F"],
                param: Some(P::LogNormal),
            },
            OmegaCase {
                input: "OMEGA(2,1) Corr CL/F-KA",
                prefix: Some("OMEGA(2,1)"),
                name: "Corr",
                refs: &["CL/F", "KA"],
                param: None,
            },
            OmegaCase {
                input: "22 Cov CL/F,V2/F ;identity",
                prefix: Some("22"),
                name: "Cov",
                refs: &["CL/F", "V2/F"],
                param: Some(P::Identity),
            },
        ];

        for case in &cases {
            assert_omega_case(case);
        }
    }

    #[test]
    fn omega_error_cases() {
        let error_inputs = ["IIV", "IIV CL KA", "IIV CL-F-KA", "IIV CL :unknown"];
        for input in error_inputs {
            assert!(parse_omega(input).is_err(), "Expected error for '{input}'");
        }
    }
}
