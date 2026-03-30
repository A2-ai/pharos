use regex::Regex;
use std::sync::LazyLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transform {
    Identity,
    LogNormal,
    Logit,
    Proportional,
    AddErr,
    LogAddErr,
}

impl Transform {
    pub fn from_comment(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "identity" | "normal" | "none" => Some(Transform::Identity),
            "lognormal" | "log_normal" | "exp" | "log" => Some(Transform::LogNormal),
            "logit" | "log_it" => Some(Transform::Logit),
            "prop" | "proportional" => Some(Transform::Proportional),
            "adderr" | "additive" | "add" => Some(Transform::AddErr),
            "logadderr" | "logadd" | "logerr" => Some(Transform::LogAddErr),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PrefixKind {
    Theta,
    Omega,
    Sigma,
}

static THETA_PREFIX_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^(THETA\d+|THETA\(\d+\)|\d+)[:\-.,]?$").unwrap());
static OMEGA_PREFIX_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^(OMEGA\d+|OMEGA\(\d+,\d+\)|\d+)[:\-.,]?$").unwrap());
static SIGMA_PREFIX_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^(SIGMA\d+|SIGMA\(\d+,\d+\)|\d+)[:\-.,]?$").unwrap());

pub(crate) fn classify_prefix(token: &str, kind: PrefixKind) -> Option<String> {
    let re = match kind {
        PrefixKind::Theta => &*THETA_PREFIX_RE,
        PrefixKind::Omega => &*OMEGA_PREFIX_RE,
        PrefixKind::Sigma => &*SIGMA_PREFIX_RE,
    };
    re.captures(token).map(|caps| caps[1].to_string())
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Type2ThetaSigma {
    pub prefix: Option<String>,
    pub name: String,
    pub unit: Option<String>,
    pub parameterization: Option<Transform>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Type2Omega {
    pub prefix: Option<String>,
    pub name: String,
    pub raw_theta_refs: Vec<String>,
    pub parameterization: Option<Transform>,
}

fn is_param_token(token: &str) -> bool {
    token.starts_with(';') || token.starts_with(':')
}

fn is_unit_token(token: &str) -> bool {
    (token.starts_with('(') && token.ends_with(')'))
        || (token.starts_with('[') && token.ends_with(']'))
}

fn extract_unit(token: &str) -> Option<String> {
    let close = if token.starts_with('(') {
        ')'
    } else if token.starts_with('[') {
        ']'
    } else {
        return None;
    };
    if token.ends_with(close) && token.len() > 2 {
        Some(token[1..token.len() - 1].to_string())
    } else {
        None
    }
}

fn parse_transform(token: &str) -> Option<Transform> {
    let s = token
        .strip_prefix(';')
        .or_else(|| token.strip_prefix(':'))?;
    Transform::from_comment(s)
}

/// [PREFIX[: | - | . | ,]] NAME [(UNIT) | [UNIT]] [;TRANSFORM | :TRANSFORM]
pub(crate) fn parse_theta_sigma(comment: &str, kind: PrefixKind) -> Option<Type2ThetaSigma> {
    let comment = comment.trim();
    if comment.is_empty() {
        return None;
    }

    let tokens: Vec<&str> = comment.split_whitespace().collect();
    let mut idx = 0;

    // Try first token as prefix
    let prefix = classify_prefix(tokens[0], kind);
    if prefix.is_some() {
        idx += 1;
    }

    // NAME is required
    if idx >= tokens.len() {
        return None;
    }
    let name_token = tokens[idx];
    if is_unit_token(name_token) || is_param_token(name_token) {
        return None;
    }
    let name = name_token.to_string();
    idx += 1;

    // Remaining tokens: unit and/or parameterization in any order
    let mut unit: Option<String> = None;
    let mut parameterization: Option<Transform> = None;

    while idx < tokens.len() {
        let token = tokens[idx];

        if is_unit_token(token) {
            if unit.is_some() {
                return None; // duplicate unit
            }
            unit = Some(extract_unit(token)?);
        } else if is_param_token(token) {
            if parameterization.is_some() {
                return None; // duplicate parameterization
            }
            parameterization = Some(parse_transform(token)?);
        } else {
            return None; // unexpected token
        }

        idx += 1;
    }

    Some(Type2ThetaSigma {
        prefix,
        name,
        unit,
        parameterization,
    })
}

/// [PREFIX[: | - | . | ,]] NAME THETA_REF_SPEC [;TRANSFORM | :TRANSFORM]
pub(crate) fn parse_omega(comment: &str) -> Option<Type2Omega> {
    let comment = comment.trim();
    if comment.is_empty() {
        return None;
    }

    let tokens: Vec<&str> = comment.split_whitespace().collect();
    let mut idx = 0;

    // Try first token as prefix
    let prefix = classify_prefix(tokens[0], PrefixKind::Omega);
    if prefix.is_some() {
        idx += 1;
    }

    // NAME is required
    if idx >= tokens.len() {
        return None;
    }
    let name_token = tokens[idx];
    if is_unit_token(name_token) || is_param_token(name_token) {
        return None;
    }
    let name = name_token.to_string();
    idx += 1;

    // Remaining tokens: exactly 1 ref token + optional param token
    let mut parameterization: Option<Transform> = None;
    let mut ref_tokens: Vec<&str> = Vec::new();

    while idx < tokens.len() {
        let token = tokens[idx];

        if is_param_token(token) {
            if parameterization.is_some() {
                return None; // duplicate parameterization
            }
            parameterization = Some(parse_transform(token)?);
        } else {
            ref_tokens.push(token);
        }

        idx += 1;
    }

    if ref_tokens.is_empty() {
        return None;
    }

    // Multiple ref tokens are only valid when they are fragments of a comma-separated
    // off-diagonal ref list such as "CL/F, KA" or "CL/F , KA".
    if ref_tokens.len() > 1 && !ref_tokens.iter().any(|token| token.contains(',')) {
        return None;
    }

    // Split by comma → 1 or 2 refs
    let raw_theta_refs: Vec<String> = ref_tokens
        .join(" ")
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    if raw_theta_refs.is_empty() || raw_theta_refs.len() > 2 {
        return None;
    }

    Some(Type2Omega {
        prefix,
        name,
        raw_theta_refs,
        parameterization,
    })
}
