use serde::{Deserialize, Serialize};

mod transforms;
mod type1;
mod type2;

use crate::comments::type1::{
    TYPE1_OMEGA_PATTERN_RE, TYPE1_SIGMA_PATTERN_RE, TYPE1_THETA_COVARIATE_RE, TYPE1_THETA_TYPE_RE,
    TYPE1_THETA_WITH_UNIT_RE,
};
pub use crate::comments::type1::{Type1Omega, Type1Sigma, Type1Theta};
pub use transforms::Transform;
use type2::{PrefixKind, parse_omega, parse_theta_sigma};
pub use type2::{Type2Omega, Type2ThetaSigma};

#[derive(Debug, Eq, PartialEq, Clone, Copy, Serialize, Deserialize)]
pub enum CommentType {
    #[serde(rename = "type1")]
    Type1,
    #[serde(rename = "type2")]
    Type2,
}

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub enum ParsedThetaComment {
    Type1(Type1Theta),
    Type2(Type2ThetaSigma),
}

impl ParsedThetaComment {
    pub fn name(&self) -> Option<String> {
        match self {
            ParsedThetaComment::Type1(t) => match t {
                Type1Theta::WithUnit { parameter, .. } | Type1Theta::Covariate { parameter } => {
                    Some(parameter.to_string())
                }
                Type1Theta::Type { typ, .. } => Some(typ.to_string()),
            },
            ParsedThetaComment::Type2(t) => Some(t.name.clone()),
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub enum ParsedOmegaComment {
    Type1(Type1Omega),
    Type2(Type2Omega),
}

impl ParsedOmegaComment {
    pub fn name(&self) -> Option<String> {
        match self {
            ParsedOmegaComment::Type1(t) => Some(format!("{} ({})", t.name, t.theta_name)),
            ParsedOmegaComment::Type2(t) => {
                Some(format!("{} ({})", t.name, t.raw_theta_refs.join(", ")))
            }
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub enum ParsedSigmaComment {
    Type1(Type1Sigma),
    Type2(Type2ThetaSigma),
}

impl ParsedSigmaComment {
    pub fn name(&self) -> Option<String> {
        match self {
            ParsedSigmaComment::Type1(t) => Some(t.name.to_string()),
            ParsedSigmaComment::Type2(t) => Some(t.name.clone()),
        }
    }
}

pub fn parse_theta_param(comment: &str, typ: CommentType) -> Option<ParsedThetaComment> {
    let comment = comment.trim();

    match typ {
        CommentType::Type1 => {
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
        CommentType::Type2 => {
            parse_theta_sigma(comment, PrefixKind::Theta).map(ParsedThetaComment::Type2)
        }
    }
}

pub fn parse_omega_param(comment: &str, typ: CommentType) -> Option<ParsedOmegaComment> {
    let comment = comment.trim();
    match typ {
        CommentType::Type1 => {
            if let Some(captures) = TYPE1_OMEGA_PATTERN_RE.captures(comment) {
                return Some(ParsedOmegaComment::Type1(Type1Omega {
                    name: captures[1].to_string(),
                    theta_name: captures[2].to_string(),
                    parameterization: captures[3].to_string(),
                }));
            }
            None
        }
        CommentType::Type2 => parse_omega(comment).map(ParsedOmegaComment::Type2),
    }
}

pub fn parse_sigma_param(comment: &str, typ: CommentType) -> Option<ParsedSigmaComment> {
    let comment = comment.trim();
    match typ {
        CommentType::Type1 => {
            if let Some(captures) = TYPE1_SIGMA_PATTERN_RE.captures(comment) {
                return Some(ParsedSigmaComment::Type1(Type1Sigma {
                    name: captures[1].to_string(),
                    parameterization: captures.get(2).map(|m| m.as_str().to_string()),
                }));
            }
            None
        }
        CommentType::Type2 => {
            parse_theta_sigma(comment, PrefixKind::Sigma).map(ParsedSigmaComment::Type2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CommentType;
    use transforms::Transform as P;

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
            // --- Type2 ---
            // Name only
            (
                "CL",
                CommentType::Type2,
                Some(ParsedThetaComment::Type2(Type2ThetaSigma {
                    prefix: None,
                    name: "CL".to_string(),
                    unit: None,
                    parameterization: None,
                })),
            ),
            // Name with unit in parens
            (
                "CL (L/day)",
                CommentType::Type2,
                Some(ParsedThetaComment::Type2(Type2ThetaSigma {
                    prefix: None,
                    name: "CL".to_string(),
                    unit: Some("L/day".to_string()),
                    parameterization: None,
                })),
            ),
            // Name with unit in brackets
            (
                "KA [1/(mg*h)]",
                CommentType::Type2,
                Some(ParsedThetaComment::Type2(Type2ThetaSigma {
                    prefix: None,
                    name: "KA".to_string(),
                    unit: Some("1/(mg*h)".to_string()),
                    parameterization: None,
                })),
            ),
            // Name with parameterization
            (
                "CL ;exp",
                CommentType::Type2,
                Some(ParsedThetaComment::Type2(Type2ThetaSigma {
                    prefix: None,
                    name: "CL".to_string(),
                    unit: None,
                    parameterization: Some(P::LogNormal),
                })),
            ),
            // Full: prefix + name + unit + param
            (
                "THETA1: CL (L/day) ;exp",
                CommentType::Type2,
                Some(ParsedThetaComment::Type2(Type2ThetaSigma {
                    prefix: Some("THETA1".to_string()),
                    name: "CL".to_string(),
                    unit: Some("L/day".to_string()),
                    parameterization: Some(P::LogNormal),
                })),
            ),
            // Bare number prefix
            (
                "1: CL (L/day) ;exp",
                CommentType::Type2,
                Some(ParsedThetaComment::Type2(Type2ThetaSigma {
                    prefix: Some("1".to_string()),
                    name: "CL".to_string(),
                    unit: Some("L/day".to_string()),
                    parameterization: Some(P::LogNormal),
                })),
            ),
            // Param before unit (flexible order)
            (
                "CL ;EXP (L/day)",
                CommentType::Type2,
                Some(ParsedThetaComment::Type2(Type2ThetaSigma {
                    prefix: None,
                    name: "CL".to_string(),
                    unit: Some("L/day".to_string()),
                    parameterization: Some(P::LogNormal),
                })),
            ),
            // Slash in name
            (
                "CL/F [L/h]",
                CommentType::Type2,
                Some(ParsedThetaComment::Type2(Type2ThetaSigma {
                    prefix: None,
                    name: "CL/F".to_string(),
                    unit: Some("L/h".to_string()),
                    parameterization: None,
                })),
            ),
            // Name starting with digit
            (
                "5FU (mg/L)",
                CommentType::Type2,
                Some(ParsedThetaComment::Type2(Type2ThetaSigma {
                    prefix: None,
                    name: "5FU".to_string(),
                    unit: Some("mg/L".to_string()),
                    parameterization: None,
                })),
            ),
            // Colon param delimiter
            (
                "AddErr [ng/mL] :ADD",
                CommentType::Type2,
                Some(ParsedThetaComment::Type2(Type2ThetaSigma {
                    prefix: None,
                    name: "AddErr".to_string(),
                    unit: Some("ng/mL".to_string()),
                    parameterization: Some(P::AddErr),
                })),
            ),
            // All transform aliases
            (
                "CL ;identity",
                CommentType::Type2,
                Some(ParsedThetaComment::Type2(Type2ThetaSigma {
                    prefix: None,
                    name: "CL".to_string(),
                    unit: None,
                    parameterization: Some(P::Identity),
                })),
            ),
            (
                "CL ;normal",
                CommentType::Type2,
                Some(ParsedThetaComment::Type2(Type2ThetaSigma {
                    prefix: None,
                    name: "CL".to_string(),
                    unit: None,
                    parameterization: Some(P::Identity),
                })),
            ),
            (
                "CL ;none",
                CommentType::Type2,
                Some(ParsedThetaComment::Type2(Type2ThetaSigma {
                    prefix: None,
                    name: "CL".to_string(),
                    unit: None,
                    parameterization: Some(P::Identity),
                })),
            ),
            (
                "CL ;log",
                CommentType::Type2,
                Some(ParsedThetaComment::Type2(Type2ThetaSigma {
                    prefix: None,
                    name: "CL".to_string(),
                    unit: None,
                    parameterization: Some(P::LogNormal),
                })),
            ),
            (
                "CL ;lognormal",
                CommentType::Type2,
                Some(ParsedThetaComment::Type2(Type2ThetaSigma {
                    prefix: None,
                    name: "CL".to_string(),
                    unit: None,
                    parameterization: Some(P::LogNormal),
                })),
            ),
            (
                "CL ;logit",
                CommentType::Type2,
                Some(ParsedThetaComment::Type2(Type2ThetaSigma {
                    prefix: None,
                    name: "CL".to_string(),
                    unit: None,
                    parameterization: Some(P::Logit),
                })),
            ),
            (
                "CL ;prop",
                CommentType::Type2,
                Some(ParsedThetaComment::Type2(Type2ThetaSigma {
                    prefix: None,
                    name: "CL".to_string(),
                    unit: None,
                    parameterization: Some(P::Proportional),
                })),
            ),
            (
                "CL ;proportional",
                CommentType::Type2,
                Some(ParsedThetaComment::Type2(Type2ThetaSigma {
                    prefix: None,
                    name: "CL".to_string(),
                    unit: None,
                    parameterization: Some(P::Proportional),
                })),
            ),
            (
                "CL ;adderr",
                CommentType::Type2,
                Some(ParsedThetaComment::Type2(Type2ThetaSigma {
                    prefix: None,
                    name: "CL".to_string(),
                    unit: None,
                    parameterization: Some(P::AddErr),
                })),
            ),
            (
                "CL ;additive",
                CommentType::Type2,
                Some(ParsedThetaComment::Type2(Type2ThetaSigma {
                    prefix: None,
                    name: "CL".to_string(),
                    unit: None,
                    parameterization: Some(P::AddErr),
                })),
            ),
            (
                "CL ;add",
                CommentType::Type2,
                Some(ParsedThetaComment::Type2(Type2ThetaSigma {
                    prefix: None,
                    name: "CL".to_string(),
                    unit: None,
                    parameterization: Some(P::AddErr),
                })),
            ),
            (
                "CL ;logadderr",
                CommentType::Type2,
                Some(ParsedThetaComment::Type2(Type2ThetaSigma {
                    prefix: None,
                    name: "CL".to_string(),
                    unit: None,
                    parameterization: Some(P::LogAddErr),
                })),
            ),
            (
                "CL ;logerr",
                CommentType::Type2,
                Some(ParsedThetaComment::Type2(Type2ThetaSigma {
                    prefix: None,
                    name: "CL".to_string(),
                    unit: None,
                    parameterization: Some(P::LogAddErr),
                })),
            ),
            // Whitespace handling
            (
                "  CL (L/h)  ",
                CommentType::Type2,
                Some(ParsedThetaComment::Type2(Type2ThetaSigma {
                    prefix: None,
                    name: "CL".to_string(),
                    unit: Some("L/h".to_string()),
                    parameterization: None,
                })),
            ),
            // Type2 invalid cases
            ("", CommentType::Type2, None),
            ("   ", CommentType::Type2, None),
            ("CL :blahblah", CommentType::Type2, None), // unknown transform
            ("CL ;exp :logit", CommentType::Type2, None), // duplicate parameterization
            ("CL (L/h) [mg]", CommentType::Type2, None), // duplicate unit
            ("not a valid comment", CommentType::Type2, None), // unexpected token
            ("THETA1: CL WT () :EXP", CommentType::Type2, None), // unexpected token + empty unit
            ("CL ()", CommentType::Type2, None),        // empty unit
            (":exp", CommentType::Type2, None),         // param token as name
            ("(L/h)", CommentType::Type2, None),        // unit token as name
            ("SIGMA2: CL", CommentType::Type2, None),   // wrong prefix kind
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
            // --- Type2 ---
            (
                "IIV CL ;exp",
                CommentType::Type2,
                Some(ParsedOmegaComment::Type2(Type2Omega {
                    prefix: None,
                    name: "IIV".to_string(),
                    raw_theta_refs: vec!["CL".to_string()],
                    parameterization: Some(P::LogNormal),
                })),
            ),
            (
                "11 IIV CL/F ;log",
                CommentType::Type2,
                Some(ParsedOmegaComment::Type2(Type2Omega {
                    prefix: Some("11".to_string()),
                    name: "IIV".to_string(),
                    raw_theta_refs: vec!["CL/F".to_string()],
                    parameterization: Some(P::LogNormal),
                })),
            ),
            // Off-diagonal: two comma-separated refs
            (
                "OMEGA(2,1) Corr CL/F, KA",
                CommentType::Type2,
                Some(ParsedOmegaComment::Type2(Type2Omega {
                    prefix: Some("OMEGA(2,1)".to_string()),
                    name: "Corr".to_string(),
                    raw_theta_refs: vec!["CL/F".to_string(), "KA".to_string()],
                    parameterization: None,
                })),
            ),
            (
                "22 Cov CL/F,V2/F ;identity",
                CommentType::Type2,
                Some(ParsedOmegaComment::Type2(Type2Omega {
                    prefix: Some("22".to_string()),
                    name: "Cov".to_string(),
                    raw_theta_refs: vec!["CL/F".to_string(), "V2/F".to_string()],
                    parameterization: Some(P::Identity),
                })),
            ),
            // Hyphenated theta ref
            (
                "33 IIV WT-on-CL ;Log",
                CommentType::Type2,
                Some(ParsedOmegaComment::Type2(Type2Omega {
                    prefix: Some("33".to_string()),
                    name: "IIV".to_string(),
                    raw_theta_refs: vec!["WT-on-CL".to_string()],
                    parameterization: Some(P::LogNormal),
                })),
            ),
            (
                "IIV CL-F-KA ;Log",
                CommentType::Type2,
                Some(ParsedOmegaComment::Type2(Type2Omega {
                    prefix: None,
                    name: "IIV".to_string(),
                    raw_theta_refs: vec!["CL-F-KA".to_string()],
                    parameterization: Some(P::LogNormal),
                })),
            ),
            // Whitespace handling
            (
                "  IIV  CL  ;exp  ",
                CommentType::Type2,
                Some(ParsedOmegaComment::Type2(Type2Omega {
                    prefix: None,
                    name: "IIV".to_string(),
                    raw_theta_refs: vec!["CL".to_string()],
                    parameterization: Some(P::LogNormal),
                })),
            ),
            // Type2 invalid cases
            ("IIV", CommentType::Type2, None),       // no theta ref
            ("IIV CL KA", CommentType::Type2, None), // ambiguous: 2 ref tokens
            ("IIV CL,F,KA", CommentType::Type2, None), // 3 comma-separated refs
            ("IIV CL :unknown", CommentType::Type2, None), // unknown transform
            ("IIV CL ;log :identity", CommentType::Type2, None), // duplicate param
            ("THETA1 IIV CL", CommentType::Type2, None), // wrong prefix kind
            ("", CommentType::Type2, None),
            ("   ", CommentType::Type2, None),
        ];

        for (input, comment_type, expected) in inputs {
            let result = parse_omega_param(input, comment_type);
            assert_eq!(result, expected, "Failed for input: '{}'", input);
        }
    }

    #[test]
    fn test_type2_omega_names_include_theta_refs() {
        let parsed = parse_omega_param("IIV CL ;exp", CommentType::Type2).unwrap();
        assert_eq!(parsed.name(), Some("IIV (CL)".to_string()));

        let parsed = parse_omega_param("Corr CL/F, KA", CommentType::Type2).unwrap();
        assert_eq!(parsed.name(), Some("Corr (CL/F, KA)".to_string()));
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
            // --- Type2 ---
            (
                "RUV",
                CommentType::Type2,
                Some(ParsedSigmaComment::Type2(Type2ThetaSigma {
                    prefix: None,
                    name: "RUV".to_string(),
                    unit: None,
                    parameterization: None,
                })),
            ),
            (
                "SIGMA2 AddErr :AddErr",
                CommentType::Type2,
                Some(ParsedSigmaComment::Type2(Type2ThetaSigma {
                    prefix: Some("SIGMA2".to_string()),
                    name: "AddErr".to_string(),
                    unit: None,
                    parameterization: Some(P::AddErr),
                })),
            ),
            (
                "11 PropErr ;proportional",
                CommentType::Type2,
                Some(ParsedSigmaComment::Type2(Type2ThetaSigma {
                    prefix: Some("11".to_string()),
                    name: "PropErr".to_string(),
                    unit: None,
                    parameterization: Some(P::Proportional),
                })),
            ),
            (
                "AddErr [ng/mL] :ADD",
                CommentType::Type2,
                Some(ParsedSigmaComment::Type2(Type2ThetaSigma {
                    prefix: None,
                    name: "AddErr".to_string(),
                    unit: Some("ng/mL".to_string()),
                    parameterization: Some(P::AddErr),
                })),
            ),
            ("", CommentType::Type2, None),
            ("CL :blahblah", CommentType::Type2, None),
            ("THETA1: AddErr", CommentType::Type2, None),
        ];

        for (input, comment_type, expected) in inputs {
            let result = parse_sigma_param(input, comment_type);
            assert_eq!(result, expected, "Failed for input: '{}'", input);
        }
    }
}
