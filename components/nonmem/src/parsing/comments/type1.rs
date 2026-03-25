use std::sync::LazyLock;

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::parsing::model::Model;

use super::{ParamName, ParsedOmegaComment, ParsedSigmaComment, ParsedThetaComment};

static TYPE1_OMEGA_PATTERN_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(OM\d+)\s+(\w+)\s+:(\w+)$").unwrap());

static TYPE1_SIGMA_PATTERN_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(SIG\d+)(?:\s+:(\w+))?$").unwrap());

static TYPE1_THETA_WITH_UNIT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^([\w/\-]+)\s+\(([^)]+)\)(?:\s+:(\w+))?$").unwrap());

static TYPE1_THETA_COVARIATE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^([\w/\-]+)\s+cov$").unwrap());

static TYPE1_THETA_TYPE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(.+?)\s+:(\w+)$").unwrap());

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub enum Type1Theta {
    /// `TVCL (L/h) :LOG` -> parameter: TVCL, unit: L/h, parametrization: LOG
    WithUnit {
        parameter: String,
        unit: String,
        parametrization: Option<String>,
    },
    /// `CRCL cov` -> parameter: CRCL
    Covariate { parameter: String },
    /// `RES ERR :stdev` -> typ: RES ERR, parameterization: stdev
    Type {
        typ: String,
        parameterization: String,
    },
    /// Anything that doesn't match one of the above types
    Unknown(String),
}

impl ParamName for Type1Theta {
    fn name(&self) -> Option<String> {
        match self {
            Type1Theta::WithUnit { parameter, .. } | Type1Theta::Covariate { parameter } => {
                Some(parameter.to_string())
            }
            Type1Theta::Type { typ, .. } => Some(typ.to_string()),
            Type1Theta::Unknown(_) => None,
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub struct Type1Omega {
    pub name: String,
    pub theta_name: String,
    pub parameterization: String,
}

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub struct Type1Sigma {
    pub name: String,
    pub parameterization: Option<String>,
}

pub fn parse_theta_param(comment: &str) -> Option<ParsedThetaComment> {
    let comment = comment.trim();

    // Try WithUnit pattern: "TVCL (L/h)" or "TVCL (L/h) :LOG"
    if let Some(captures) = TYPE1_THETA_WITH_UNIT_RE.captures(comment) {
        return Some(ParsedThetaComment::Type1(Type1Theta::WithUnit {
            parameter: captures[1].to_string(),
            unit: captures[2].to_string(),
            parametrization: captures.get(3).map(|m| m.as_str().to_string()),
        }));
    }

    // Try Covariate pattern: "CRCL cov"
    if let Some(captures) = TYPE1_THETA_COVARIATE_RE.captures(comment) {
        return Some(ParsedThetaComment::Type1(Type1Theta::Covariate {
            parameter: captures[1].to_string(),
        }));
    }

    // Try Type pattern: "RES ERR :stdev"
    if let Some(captures) = TYPE1_THETA_TYPE_RE.captures(comment) {
        return Some(ParsedThetaComment::Type1(Type1Theta::Type {
            typ: captures[1].to_string(),
            parameterization: captures[2].to_string(),
        }));
    }

    None
}

pub fn parse_omega_param(comment: &str) -> Option<ParsedOmegaComment> {
    let comment = comment.trim();

    // Try Omega pattern: "OM1 TVCL :EXP"
    if let Some(captures) = TYPE1_OMEGA_PATTERN_RE.captures(comment) {
        return Some(ParsedOmegaComment::Type1(Type1Omega {
            name: captures[1].to_string(),
            theta_name: captures[2].to_string(),
            parameterization: captures[3].to_string(),
        }));
    }

    None
}

pub fn parse_sigma_param(comment: &str) -> Option<ParsedSigmaComment> {
    let comment = comment.trim();

    // Try Sigma pattern: "SIG1" or "SIG1 :OMIT_TBL"
    if let Some(captures) = TYPE1_SIGMA_PATTERN_RE.captures(comment) {
        return Some(ParsedSigmaComment::Type1(Type1Sigma {
            name: captures[1].to_string(),
            parameterization: captures.get(2).map(|m| m.as_str().to_string()),
        }));
    }

    None
}

pub fn parse_comments(model: &mut Model) -> Vec<String> {
    let mut out = Vec::new();
    for theta in model.theta_parameters.iter_mut() {
        if let Some(c) = theta.comment.as_ref() {
            theta.parsed_comment = parse_theta_param(c.as_str());
            if theta.parsed_comment.is_none() {
                out.push(c.to_string());
            }
        }
    }

    for block in model.omega_blocks.iter_mut() {
        for p in block.parameters.iter_mut() {
            if let Some(c) = p.comment.as_ref() {
                p.parsed_comment = parse_omega_param(c.as_str());
                if p.parsed_comment.is_none() {
                    out.push(c.to_string());
                }
            }
        }
    }

    for block in model.sigma_blocks.iter_mut() {
        for p in block.parameters.iter_mut() {
            if let Some(c) = p.comment.as_ref() {
                p.parsed_comment = parse_sigma_param(c.as_str());
                if p.parsed_comment.is_none() {
                    out.push(c.to_string());
                }
            }
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_theta_param() {
        let inputs = vec![
            // WithUnit pattern - valid cases
            (
                "TVCL (L/h)",
                Some(ParsedThetaComment::Type1(Type1Theta::WithUnit {
                    parameter: "TVCL".to_string(),
                    unit: "L/h".to_string(),
                    parametrization: None,
                })),
            ),
            (
                "TVKA (1/h)",
                Some(ParsedThetaComment::Type1(Type1Theta::WithUnit {
                    parameter: "TVKA".to_string(),
                    unit: "1/h".to_string(),
                    parametrization: None,
                })),
            ),
            (
                "TVKA (1/h) :LOG",
                Some(ParsedThetaComment::Type1(Type1Theta::WithUnit {
                    parameter: "TVKA".to_string(),
                    unit: "1/h".to_string(),
                    parametrization: Some("LOG".to_string()),
                })),
            ),
            (
                "CL (L/h/kg)",
                Some(ParsedThetaComment::Type1(Type1Theta::WithUnit {
                    parameter: "CL".to_string(),
                    unit: "L/h/kg".to_string(),
                    parametrization: None,
                })),
            ),
            (
                "  TVCL (L/h)  ",
                Some(ParsedThetaComment::Type1(Type1Theta::WithUnit {
                    parameter: "TVCL".to_string(),
                    unit: "L/h".to_string(),
                    parametrization: None,
                })),
            ),
            // Covariate pattern - valid cases
            (
                "CRCL cov",
                Some(ParsedThetaComment::Type1(Type1Theta::Covariate {
                    parameter: "CRCL".to_string(),
                })),
            ),
            (
                "AGE cov",
                Some(ParsedThetaComment::Type1(Type1Theta::Covariate {
                    parameter: "AGE".to_string(),
                })),
            ),
            (
                "WT cov",
                Some(ParsedThetaComment::Type1(Type1Theta::Covariate {
                    parameter: "WT".to_string(),
                })),
            ),
            (
                "  CRCL cov  ",
                Some(ParsedThetaComment::Type1(Type1Theta::Covariate {
                    parameter: "CRCL".to_string(),
                })),
            ),
            // Type pattern - valid cases
            (
                "RES ERR :stdev",
                Some(ParsedThetaComment::Type1(Type1Theta::Type {
                    typ: "RES ERR".to_string(),
                    parameterization: "stdev".to_string(),
                })),
            ),
            (
                "PROP ERR :var",
                Some(ParsedThetaComment::Type1(Type1Theta::Type {
                    typ: "PROP ERR".to_string(),
                    parameterization: "var".to_string(),
                })),
            ),
            (
                "ADD ERR :stdev",
                Some(ParsedThetaComment::Type1(Type1Theta::Type {
                    typ: "ADD ERR".to_string(),
                    parameterization: "stdev".to_string(),
                })),
            ),
            (
                "  RES ERR :stdev  ",
                Some(ParsedThetaComment::Type1(Type1Theta::Type {
                    typ: "RES ERR".to_string(),
                    parameterization: "stdev".to_string(),
                })),
            ),
            // Invalid cases - should return None
            ("invalid", None),
            ("TVCL", None),
            ("(L/h)", None),
            ("cov", None),
            (":stdev", None),
            ("", None),
            ("   ", None),
            ("TVCL (", None),
            ("TVCL )", None),
            ("TVCL ()", None),
        ];

        for (input, expected) in inputs {
            let result = parse_theta_param(input);
            assert_eq!(result, expected, "Failed for input: '{}'", input);
        }
    }

    #[test]
    fn test_parse_omega_param() {
        let inputs = vec![
            // Valid omega patterns
            (
                "OM1 TVCL :EXP",
                Some(ParsedOmegaComment::Type1(Type1Omega {
                    name: "OM1".to_string(),
                    theta_name: "TVCL".to_string(),
                    parameterization: "EXP".to_string(),
                })),
            ),
            (
                "OM2 TVKA :OMIT_TBL",
                Some(ParsedOmegaComment::Type1(Type1Omega {
                    name: "OM2".to_string(),
                    theta_name: "TVKA".to_string(),
                    parameterization: "OMIT_TBL".to_string(),
                })),
            ),
            (
                "OM10 CL :LOG",
                Some(ParsedOmegaComment::Type1(Type1Omega {
                    name: "OM10".to_string(),
                    theta_name: "CL".to_string(),
                    parameterization: "LOG".to_string(),
                })),
            ),
            (
                "OM3 V1 :VAR",
                Some(ParsedOmegaComment::Type1(Type1Omega {
                    name: "OM3".to_string(),
                    theta_name: "V1".to_string(),
                    parameterization: "VAR".to_string(),
                })),
            ),
            (
                "  OM1 TVCL :EXP  ",
                Some(ParsedOmegaComment::Type1(Type1Omega {
                    name: "OM1".to_string(),
                    theta_name: "TVCL".to_string(),
                    parameterization: "EXP".to_string(),
                })),
            ),
            // Invalid cases - should return None
            ("OMEGA1 TVCL :EXP", None),
            ("OM1 :EXP", None),
            ("OM1 TVCL", None),
            ("OM1 TVCL :", None),
            ("1 TVCL :EXP", None),
            ("OM TVCL :EXP", None),
            ("invalid", None),
            ("", None),
            ("   ", None),
            (
                "OM1  TVCL  :EXP",
                Some(ParsedOmegaComment::Type1(Type1Omega {
                    name: "OM1".to_string(),
                    theta_name: "TVCL".to_string(),
                    parameterization: "EXP".to_string(),
                })),
            ),
        ];

        for (input, expected) in inputs {
            let result = parse_omega_param(input);
            assert_eq!(result, expected, "Failed for input: '{}'", input);
        }
    }

    #[test]
    fn test_parse_sigma_param() {
        let inputs = vec![
            // Valid sigma patterns with parameterization
            (
                "SIG1 :OMIT_TBL",
                Some(ParsedSigmaComment::Type1(Type1Sigma {
                    name: "SIG1".to_string(),
                    parameterization: Some("OMIT_TBL".to_string()),
                })),
            ),
            (
                "SIG2 :EXP",
                Some(ParsedSigmaComment::Type1(Type1Sigma {
                    name: "SIG2".to_string(),
                    parameterization: Some("EXP".to_string()),
                })),
            ),
            (
                "SIG10 :LOG",
                Some(ParsedSigmaComment::Type1(Type1Sigma {
                    name: "SIG10".to_string(),
                    parameterization: Some("LOG".to_string()),
                })),
            ),
            (
                "SIG3 :VAR",
                Some(ParsedSigmaComment::Type1(Type1Sigma {
                    name: "SIG3".to_string(),
                    parameterization: Some("VAR".to_string()),
                })),
            ),
            (
                "SIG5 :STDEV",
                Some(ParsedSigmaComment::Type1(Type1Sigma {
                    name: "SIG5".to_string(),
                    parameterization: Some("STDEV".to_string()),
                })),
            ),
            (
                "  SIG1 :OMIT_TBL  ",
                Some(ParsedSigmaComment::Type1(Type1Sigma {
                    name: "SIG1".to_string(),
                    parameterization: Some("OMIT_TBL".to_string()),
                })),
            ),
            (
                "SIG1  :EXP",
                Some(ParsedSigmaComment::Type1(Type1Sigma {
                    name: "SIG1".to_string(),
                    parameterization: Some("EXP".to_string()),
                })),
            ),
            // Valid sigma patterns without parameterization
            (
                "SIG1",
                Some(ParsedSigmaComment::Type1(Type1Sigma {
                    name: "SIG1".to_string(),
                    parameterization: None,
                })),
            ),
            (
                "SIG2",
                Some(ParsedSigmaComment::Type1(Type1Sigma {
                    name: "SIG2".to_string(),
                    parameterization: None,
                })),
            ),
            (
                "SIG10",
                Some(ParsedSigmaComment::Type1(Type1Sigma {
                    name: "SIG10".to_string(),
                    parameterization: None,
                })),
            ),
            (
                "  SIG5  ",
                Some(ParsedSigmaComment::Type1(Type1Sigma {
                    name: "SIG5".to_string(),
                    parameterization: None,
                })),
            ),
            // Invalid cases - should return None
            ("SIGMA1 :EXP", None),
            ("SIG1 :", None),
            ("1 :EXP", None),
            ("SIG :EXP", None),
            ("SIG", None),
            (":OMIT_TBL", None),
            ("invalid", None),
            ("", None),
            ("   ", None),
        ];

        for (input, expected) in inputs {
            let result = parse_sigma_param(input);
            assert_eq!(result, expected, "Failed for input: '{}'", input);
        }
    }
}
