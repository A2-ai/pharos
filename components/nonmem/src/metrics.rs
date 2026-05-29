use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct InformationCriteria {
    pub n_estimated_parameters: usize,
    pub n_observations: usize,
    pub aic: f64,
    pub bic: f64,
}

impl InformationCriteria {
    pub fn new(ofv: f64, n_estimated_parameters: usize, n_observations: usize) -> Self {
        Self {
            n_estimated_parameters,
            n_observations,
            aic: aic(ofv, n_estimated_parameters),
            bic: bic(ofv, n_estimated_parameters, n_observations),
        }
    }
}

/// AIC = -2 log(L(theta)) + 2|theta|
/// where:
///     nonmem's OFV = -2 log(L(theta)) (log likelihood)
///     |theta| is length of estimated (non-fixed) parameters
pub fn aic(ofv: f64, n_estimated_parameters: usize) -> f64 {
    let k = n_estimated_parameters as f64;
    ofv + 2.0 * k
}

/// BIC = -2 log(L(theta)) + |theta| ln(n)
/// where:
///     nonmem's OFV = -2 log(L(theta)) (log likelihood)
///     |theta| is length of estimated (non-fixed) parameters
///     n: number of observations
pub fn bic(ofv: f64, n_estimated_parameters: usize, n_observations: usize) -> f64 {
    let k = n_estimated_parameters as f64;
    let n = n_observations as f64;
    ofv + k * n.ln()
}
