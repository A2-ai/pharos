use regex::Regex;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::sync::LazyLock;

use super::Transform;

// Regex patterns for Type1 comment parsing
pub(crate) static TYPE1_OMEGA_PATTERN_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(OM\d+)\s+(\w+)\s+:(\w+)$").unwrap());

pub(crate) static TYPE1_SIGMA_PATTERN_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(SIG\d+)(?:\s+:(\w+))?$").unwrap());

pub(crate) static TYPE1_THETA_WITH_UNIT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^([\w/\-]+)\s+\(([^)]+)\)(?:\s+:(\w+))?$").unwrap());

pub(crate) static TYPE1_THETA_COVARIATE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^([\w/\-]+)\s+cov$").unwrap());

pub(crate) static TYPE1_THETA_TYPE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(.+?)\s+:(\w+)$").unwrap());

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub enum Type1Theta {
    /// `TVCL (L/h) :LOG` -> parameter: TVCL, unit: L/h, parameterization: LOG
    WithUnit {
        parameter: String,
        unit: String,
        parameterization: Option<String>,
    },
    /// `CRCL cov` -> parameter: CRCL
    Covariate { parameter: String },
    /// `RES ERR :stdev` -> typ: RES ERR, parameterization: stdev
    Type {
        typ: String,
        parameterization: String,
    },
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

impl Type1Theta {
    pub fn transform(&self) -> Option<Transform> {
        match self {
            Type1Theta::WithUnit {
                parameterization, ..
            } => parameterization
                .as_deref()
                .and_then(|tag| Transform::from_str(tag).ok()),
            Type1Theta::Covariate { .. } => None,
            Type1Theta::Type {
                parameterization, ..
            } => Transform::from_str(parameterization).ok(),
        }
    }
}

impl Type1Omega {
    pub fn transform(&self) -> Option<Transform> {
        Transform::from_str(&self.parameterization).ok()
    }
}

impl Type1Sigma {
    pub fn transform(&self) -> Option<Transform> {
        self.parameterization
            .as_deref()
            .and_then(|tag| Transform::from_str(tag).ok())
    }
}
