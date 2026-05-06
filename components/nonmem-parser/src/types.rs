use std::fmt;
use std::str::FromStr;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum ParameterType {
    Theta,
    Omega,
    Sigma,
}

impl ParameterType {
    /// Returns the prefix used for fixed and random effect labels (ETA for Omega, EPS for Sigma)
    pub fn prefix(&self) -> &'static str {
        match self {
            ParameterType::Theta => "THETA",
            ParameterType::Omega => "ETA",
            ParameterType::Sigma => "EPS",
        }
    }
}

impl fmt::Display for ParameterType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ParameterType::Theta => write!(f, "THETA"),
            ParameterType::Omega => write!(f, "OMEGA"),
            ParameterType::Sigma => write!(f, "SIGMA"),
        }
    }
}

impl FromStr for ParameterType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.to_lowercase().starts_with("omega") {
            Ok(ParameterType::Omega)
        } else if s.to_lowercase().starts_with("sigma") {
            Ok(ParameterType::Sigma)
        } else {
            Ok(ParameterType::Theta)
        }
    }
}
