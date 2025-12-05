use std::sync::LazyLock;

use config::CommentType;
use regex::Regex;
use serde::{Deserialize, Serialize, de::DeserializeOwned};

// Regex patterns for Type1 comment parsing
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

pub trait ParamName: Serialize + DeserializeOwned + Clone {
    fn name(&self) -> Option<String>;
}

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

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub enum ParsedThetaComment {
    Type1(Type1Theta),
}

impl ParamName for ParsedThetaComment {
    fn name(&self) -> Option<String> {
        match self {
            ParsedThetaComment::Type1(t) => t.name(),
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub enum ParsedOmegaComment {
    Type1(Type1Omega),
}

impl ParamName for ParsedOmegaComment {
    fn name(&self) -> Option<String> {
        match self {
            ParsedOmegaComment::Type1(t) => Some(format!("{} ({})", t.name, t.theta_name)),
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub enum ParsedSigmaComment {
    Type1(Type1Sigma),
}

impl ParamName for ParsedSigmaComment {
    fn name(&self) -> Option<String> {
        match self {
            ParsedSigmaComment::Type1(t) => Some(t.name.to_string()),
        }
    }
}

pub fn parse_theta_param(comment: &str, typ: CommentType) -> Option<ParsedThetaComment> {
    let comment = comment.trim();

    if typ == CommentType::Type1 {
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
    }

    None
}

pub fn parse_omega_param(comment: &str, typ: CommentType) -> Option<ParsedOmegaComment> {
    let comment = comment.trim();
    if typ == CommentType::Type1 {
        // Try Omega pattern: "OM1 TVCL :EXP"
        if let Some(captures) = TYPE1_OMEGA_PATTERN_RE.captures(comment) {
            return Some(ParsedOmegaComment::Type1(Type1Omega {
                name: captures[1].to_string(),
                theta_name: captures[2].to_string(),
                parameterization: captures[3].to_string(),
            }));
        }
    }

    None
}

pub fn parse_sigma_param(comment: &str, typ: CommentType) -> Option<ParsedSigmaComment> {
    let comment = comment.trim();
    if typ == CommentType::Type1 {
        // Try Sigma pattern: "SIG1" or "SIG1 :OMIT_TBL"
        if let Some(captures) = TYPE1_SIGMA_PATTERN_RE.captures(comment) {
            return Some(ParsedSigmaComment::Type1(Type1Sigma {
                name: captures[1].to_string(),
                parameterization: captures.get(2).map(|m| m.as_str().to_string()),
            }));
        }
    }

    None
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
                CommentType::Type1,
                Some(ParsedThetaComment::Type1(Type1Theta::WithUnit {
                    parameter: "TVCL".to_string(),
                    unit: "L/h".to_string(),
                    parametrization: None,
                })),
            ),
            (
                "TVKA (1/h)",
                CommentType::Type1,
                Some(ParsedThetaComment::Type1(Type1Theta::WithUnit {
                    parameter: "TVKA".to_string(),
                    unit: "1/h".to_string(),
                    parametrization: None,
                })),
            ),
            (
                "TVKA (1/h) :LOG",
                CommentType::Type1,
                Some(ParsedThetaComment::Type1(Type1Theta::WithUnit {
                    parameter: "TVKA".to_string(),
                    unit: "1/h".to_string(),
                    parametrization: Some("LOG".to_string()),
                })),
            ),
            (
                "CL (L/h/kg)",
                CommentType::Type1,
                Some(ParsedThetaComment::Type1(Type1Theta::WithUnit {
                    parameter: "CL".to_string(),
                    unit: "L/h/kg".to_string(),
                    parametrization: None,
                })),
            ),
            (
                "  TVCL (L/h)  ",
                CommentType::Type1,
                Some(ParsedThetaComment::Type1(Type1Theta::WithUnit {
                    parameter: "TVCL".to_string(),
                    unit: "L/h".to_string(),
                    parametrization: None,
                })),
            ),
            // Covariate pattern - valid cases
            (
                "CRCL cov",
                CommentType::Type1,
                Some(ParsedThetaComment::Type1(Type1Theta::Covariate {
                    parameter: "CRCL".to_string(),
                })),
            ),
            (
                "AGE cov",
                CommentType::Type1,
                Some(ParsedThetaComment::Type1(Type1Theta::Covariate {
                    parameter: "AGE".to_string(),
                })),
            ),
            (
                "WT cov",
                CommentType::Type1,
                Some(ParsedThetaComment::Type1(Type1Theta::Covariate {
                    parameter: "WT".to_string(),
                })),
            ),
            (
                "  CRCL cov  ",
                CommentType::Type1,
                Some(ParsedThetaComment::Type1(Type1Theta::Covariate {
                    parameter: "CRCL".to_string(),
                })),
            ),
            // Type pattern - valid cases
            (
                "RES ERR :stdev",
                CommentType::Type1,
                Some(ParsedThetaComment::Type1(Type1Theta::Type {
                    typ: "RES ERR".to_string(),
                    parameterization: "stdev".to_string(),
                })),
            ),
            (
                "PROP ERR :var",
                CommentType::Type1,
                Some(ParsedThetaComment::Type1(Type1Theta::Type {
                    typ: "PROP ERR".to_string(),
                    parameterization: "var".to_string(),
                })),
            ),
            (
                "ADD ERR :stdev",
                CommentType::Type1,
                Some(ParsedThetaComment::Type1(Type1Theta::Type {
                    typ: "ADD ERR".to_string(),
                    parameterization: "stdev".to_string(),
                })),
            ),
            (
                "  RES ERR :stdev  ",
                CommentType::Type1,
                Some(ParsedThetaComment::Type1(Type1Theta::Type {
                    typ: "RES ERR".to_string(),
                    parameterization: "stdev".to_string(),
                })),
            ),
            // Invalid cases - should return None
            ("invalid", CommentType::Type1, None),
            ("TVCL", CommentType::Type1, None), // missing unit or pattern
            ("(L/h)", CommentType::Type1, None), // missing parameter
            ("cov", CommentType::Type1, None),  // missing parameter
            (":stdev", CommentType::Type1, None), // missing type
            ("", CommentType::Type1, None),     // empty string
            ("   ", CommentType::Type1, None),  // only whitespace
            ("TVCL (", CommentType::Type1, None), // malformed unit
            ("TVCL )", CommentType::Type1, None), // malformed unit
            ("TVCL ()", CommentType::Type1, None), // empty unit
        ];

        for (input, comment_type, expected) in inputs {
            let result = parse_theta_param(input, comment_type);
            assert_eq!(result, expected, "Failed for input: '{}'", input);
        }
    }

    #[test]
    fn test_parse_omega_param() {
        let inputs = vec![
            // Valid omega patterns
            (
                "OM1 TVCL :EXP",
                CommentType::Type1,
                Some(ParsedOmegaComment::Type1(Type1Omega {
                    name: "OM1".to_string(),
                    theta_name: "TVCL".to_string(),
                    parameterization: "EXP".to_string(),
                })),
            ),
            (
                "OM2 TVKA :OMIT_TBL",
                CommentType::Type1,
                Some(ParsedOmegaComment::Type1(Type1Omega {
                    name: "OM2".to_string(),
                    theta_name: "TVKA".to_string(),
                    parameterization: "OMIT_TBL".to_string(),
                })),
            ),
            (
                "OM10 CL :LOG",
                CommentType::Type1,
                Some(ParsedOmegaComment::Type1(Type1Omega {
                    name: "OM10".to_string(),
                    theta_name: "CL".to_string(),
                    parameterization: "LOG".to_string(),
                })),
            ),
            (
                "OM3 V1 :VAR",
                CommentType::Type1,
                Some(ParsedOmegaComment::Type1(Type1Omega {
                    name: "OM3".to_string(),
                    theta_name: "V1".to_string(),
                    parameterization: "VAR".to_string(),
                })),
            ),
            (
                "  OM1 TVCL :EXP  ",
                CommentType::Type1,
                Some(ParsedOmegaComment::Type1(Type1Omega {
                    name: "OM1".to_string(),
                    theta_name: "TVCL".to_string(),
                    parameterization: "EXP".to_string(),
                })),
            ),
            // Invalid cases - should return None
            ("OMEGA1 TVCL :EXP", CommentType::Type1, None), // wrong prefix
            ("OM1 :EXP", CommentType::Type1, None),         // missing theta name
            ("OM1 TVCL", CommentType::Type1, None),         // missing parameterization
            ("OM1 TVCL :", CommentType::Type1, None),       // empty parameterization
            ("1 TVCL :EXP", CommentType::Type1, None),      // missing OM prefix
            ("OM TVCL :EXP", CommentType::Type1, None),     // missing number
            ("invalid", CommentType::Type1, None),
            ("", CommentType::Type1, None),    // empty string
            ("   ", CommentType::Type1, None), // only whitespace
            (
                "OM1  TVCL  :EXP",
                CommentType::Type1,
                Some(ParsedOmegaComment::Type1(Type1Omega {
                    name: "OM1".to_string(),
                    theta_name: "TVCL".to_string(),
                    parameterization: "EXP".to_string(),
                })),
            ), // multiple spaces should still work
        ];

        for (input, comment_type, expected) in inputs {
            let result = parse_omega_param(input, comment_type);
            assert_eq!(result, expected, "Failed for input: '{}'", input);
        }
    }

    #[test]
    fn test_parse_sigma_param() {
        let inputs = vec![
            // Valid sigma patterns with parameterization
            (
                "SIG1 :OMIT_TBL",
                CommentType::Type1,
                Some(ParsedSigmaComment::Type1(Type1Sigma {
                    name: "SIG1".to_string(),
                    parameterization: Some("OMIT_TBL".to_string()),
                })),
            ),
            (
                "SIG2 :EXP",
                CommentType::Type1,
                Some(ParsedSigmaComment::Type1(Type1Sigma {
                    name: "SIG2".to_string(),
                    parameterization: Some("EXP".to_string()),
                })),
            ),
            (
                "SIG10 :LOG",
                CommentType::Type1,
                Some(ParsedSigmaComment::Type1(Type1Sigma {
                    name: "SIG10".to_string(),
                    parameterization: Some("LOG".to_string()),
                })),
            ),
            (
                "SIG3 :VAR",
                CommentType::Type1,
                Some(ParsedSigmaComment::Type1(Type1Sigma {
                    name: "SIG3".to_string(),
                    parameterization: Some("VAR".to_string()),
                })),
            ),
            (
                "SIG5 :STDEV",
                CommentType::Type1,
                Some(ParsedSigmaComment::Type1(Type1Sigma {
                    name: "SIG5".to_string(),
                    parameterization: Some("STDEV".to_string()),
                })),
            ),
            (
                "  SIG1 :OMIT_TBL  ",
                CommentType::Type1,
                Some(ParsedSigmaComment::Type1(Type1Sigma {
                    name: "SIG1".to_string(),
                    parameterization: Some("OMIT_TBL".to_string()),
                })),
            ),
            (
                "SIG1  :EXP",
                CommentType::Type1,
                Some(ParsedSigmaComment::Type1(Type1Sigma {
                    name: "SIG1".to_string(),
                    parameterization: Some("EXP".to_string()),
                })),
            ), // multiple spaces should still work
            // Valid sigma patterns without parameterization
            (
                "SIG1",
                CommentType::Type1,
                Some(ParsedSigmaComment::Type1(Type1Sigma {
                    name: "SIG1".to_string(),
                    parameterization: None,
                })),
            ),
            (
                "SIG2",
                CommentType::Type1,
                Some(ParsedSigmaComment::Type1(Type1Sigma {
                    name: "SIG2".to_string(),
                    parameterization: None,
                })),
            ),
            (
                "SIG10",
                CommentType::Type1,
                Some(ParsedSigmaComment::Type1(Type1Sigma {
                    name: "SIG10".to_string(),
                    parameterization: None,
                })),
            ),
            (
                "  SIG5  ",
                CommentType::Type1,
                Some(ParsedSigmaComment::Type1(Type1Sigma {
                    name: "SIG5".to_string(),
                    parameterization: None,
                })),
            ),
            // Invalid cases - should return None
            ("SIGMA1 :EXP", CommentType::Type1, None), // wrong prefix
            ("SIG1 :", CommentType::Type1, None),      // empty parameterization
            ("1 :EXP", CommentType::Type1, None),      // missing SIG prefix
            ("SIG :EXP", CommentType::Type1, None),    // missing number
            ("SIG", CommentType::Type1, None),         // missing number
            (":OMIT_TBL", CommentType::Type1, None),   // missing name
            ("invalid", CommentType::Type1, None),
            ("", CommentType::Type1, None),    // empty string
            ("   ", CommentType::Type1, None), // only whitespace
        ];

        for (input, comment_type, expected) in inputs {
            let result = parse_sigma_param(input, comment_type);
            assert_eq!(result, expected, "Failed for input: '{}'", input);
        }
    }
}
