mod finalize;
mod parse;

use serde::{Deserialize, Serialize};

use super::{ParamName, ParamPrefix};
use crate::parsing::model::Model;
use crate::transforms::Transform;

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize, Default)]
pub struct Type2ThetaSigma {
    // Optional prefix with numeral or nonmem name
    // THETAX, OMEGAYY, SIGMA(X,Y), X,
    // can have optional separator ,/:/.
    pub prefix: Option<String>,
    // Required name of parameter
    pub name: String,
    // Optional unit placed within () or []
    pub unit: Option<String>,
    // Optional parameterization following
    // a separator ;/:
    // ;Log :EXP :Logit :Identity
    pub parameterization: Option<Transform>,
}

impl ParamName for Type2ThetaSigma {
    fn name(&self) -> Option<String> {
        Some(self.name.clone())
    }
}

impl ParamPrefix for Type2ThetaSigma {
    fn prefix(&self) -> Option<&str> {
        self.prefix.as_deref()
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize, Default)]
pub struct Type2Omega {
    // Optional prefix with numeral or nonmem name
    // THETAX, OMEGAYY, SIGMA(X,Y), X,
    // can have optional separator ,/:/;/.
    pub prefix: Option<String>,
    // Required name of parameter
    pub name: String,
    // Required associated theta. Validated
    // to be a known theta name
    pub associated_theta: Option<Vec<String>>,
    // Optional parameterization following
    // a separator ;/:
    // ;Log :EXP :Logit :Identity
    pub parameterization: Option<Transform>,
}

impl ParamName for Type2Omega {
    fn name(&self) -> Option<String> {
        let assoc = self.associated_theta.as_deref().filter(|a| !a.is_empty());
        match assoc {
            Some(refs) => Some(format!("{} ({})", self.name, refs.join(","))),
            None => Some(self.name.clone()),
        }
    }
}

impl ParamPrefix for Type2Omega {
    fn prefix(&self) -> Option<&str> {
        self.prefix.as_deref()
    }
}

pub fn parse_comments(model: &mut Model) -> Vec<String> {
    // Parse raw comments
    let parse::ParsedComments {
        thetas,
        omegas,
        sigmas,
        mut errors,
    } = parse::parse_all(model);

    // Validate unresolved omegas
    finalize::finalize_and_apply(model, thetas, omegas, sigmas, &mut errors);

    errors
}
