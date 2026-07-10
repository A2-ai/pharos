use serde::{Deserialize, Serialize};
use std::fmt;
use std::fmt::Formatter;
use std::str::FromStr;

#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize, Default)]
pub enum EstimationMethod {
    #[default]
    Fo,
    Foce,
    Saem,
    Bayes,
    Imp,
    ImpMap,
    Its,
    Nuts,
}

impl fmt::Display for EstimationMethod {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            EstimationMethod::Fo => f.write_str("FO"),
            EstimationMethod::Foce => f.write_str("FOCE"),
            EstimationMethod::Saem => f.write_str("SAEM"),
            EstimationMethod::Bayes => f.write_str("Bayes"),
            EstimationMethod::Imp => f.write_str("IMP"),
            EstimationMethod::ImpMap => f.write_str("IMPMAP"),
            EstimationMethod::Its => f.write_str("ITS"),
            EstimationMethod::Nuts => f.write_str("NUTS"),
        }
    }
}

impl FromStr for EstimationMethod {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // NONMEM can suffix the method with "(No Prior)" or "(Evaluation)" (a
        // MAXEVAL=0 likelihood evaluation); strip both before matching the base name.
        match s
            .to_uppercase()
            .replace("(NO PRIOR)", "")
            .replace("(EVALUATION)", "")
            .trim()
        {
            "0" | "FO" | "FIRST ORDER" | "FIRST ORDER WITH INTERACTION" => Ok(EstimationMethod::Fo),
            // Conditional estimation family. Laplacian is a conditional-estimation
            // variant (not Bayesian), so it maps here rather than to Bayes.
            "1"
            | "FOCE"
            | "COND"
            | "FIRST ORDER CONDITIONAL ESTIMATION"
            | "FIRST ORDER CONDITIONAL ESTIMATION WITH INTERACTION"
            | "LAPLACE"
            | "LAPLACIAN CONDITIONAL ESTIMATION"
            | "LAPLACIAN CONDITIONAL ESTIMATION WITH INTERACTION" => Ok(EstimationMethod::Foce),
            "SAEM" | "STOCHASTIC APPROXIMATION EXPECTATION-MAXIMIZATION" => {
                Ok(EstimationMethod::Saem)
            }
            "BAYES" | "MCMC BAYESIAN ANALYSIS" => Ok(EstimationMethod::Bayes),
            "IMP"
            | "IMPORTANCE SAMPLING"
            | "OBJECTIVE FUNCTION EVALUATION BY IMPORTANCE SAMPLING" => Ok(EstimationMethod::Imp),
            "IMPMAP" | "IMPORTANCE SAMPLING ASSISTED BY MAP ESTIMATION" => {
                Ok(EstimationMethod::ImpMap)
            }
            "ITS" | "ITERATIVE TWO STAGE" => Ok(EstimationMethod::Its),
            "NUTS" | "NUTS BAYESIAN ANALYSIS" => Ok(EstimationMethod::Nuts),
            _ => Err(format!("Unknown estimation method: {s}")),
        }
    }
}

#[inline]
pub(crate) fn extract_estimation_method(line: &str) -> Option<EstimationMethod> {
    let parts: Vec<&str> = line.split(':').collect();
    if parts.len() >= 2 {
        EstimationMethod::from_str(parts[1].trim()).ok()
    } else {
        None
    }
}
