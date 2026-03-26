mod type1;
pub mod type2;

use config::CommentType;
use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::parsing::model::Model;

pub use type1::{Type1Omega, Type1Sigma, Type1Theta};
pub use type2::{Type2Omega, Type2ThetaSigma};

pub trait ParamName: Serialize + DeserializeOwned + Clone {
    fn name(&self) -> Option<String>;
}

pub trait ParamPrefix {
    fn prefix(&self) -> Option<&str>;
}

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub enum ParsedThetaComment {
    Type1(Type1Theta),
    Type2(Type2ThetaSigma),
}

impl ParamName for ParsedThetaComment {
    fn name(&self) -> Option<String> {
        match self {
            ParsedThetaComment::Type1(t) => t.name(),
            ParsedThetaComment::Type2(t) => t.name(),
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub enum ParsedOmegaComment {
    Type1(Type1Omega),
    Type2(Type2Omega),
}

impl ParamName for ParsedOmegaComment {
    fn name(&self) -> Option<String> {
        match self {
            ParsedOmegaComment::Type1(t) => Some(format!("{} ({})", t.name, t.theta_name)),
            ParsedOmegaComment::Type2(t) => t.name(),
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub enum ParsedSigmaComment {
    Type1(Type1Sigma),
    Type2(Type2ThetaSigma),
}

impl ParamName for ParsedSigmaComment {
    fn name(&self) -> Option<String> {
        match self {
            ParsedSigmaComment::Type1(t) => Some(t.name.to_string()),
            ParsedSigmaComment::Type2(t) => t.name(),
        }
    }
}

pub fn parse_comments(model: &mut Model, typ: CommentType) -> Vec<String> {
    match typ {
        CommentType::Type1 => type1::parse_comments(model),
        CommentType::Type2 => type2::parse_comments(model),
    }
}
