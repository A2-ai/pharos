use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct InformationCriteria {
    pub k: usize,
    pub n: usize,
    pub aic: f64,
    pub bic: f64,
}

impl InformationCriteria {
    pub fn new(ofv: f64, k: usize, n: usize) -> Self {
        Self {
            k,
            n,
            aic: aic(ofv, k),
            bic: bic(ofv, k, n),
        }
    }
}

/// AIC = -2 log(L(theta)) + 2|theta|
/// where:
///     nonmem's OFV = -2 log(L(theta)) (log likelihood)
///     |theta| is length of estimated (non-fixed) parameters
pub fn aic(ofv: f64, n_params: usize) -> f64 {
    let k = n_params as f64;
    ofv + 2.0 * k
}

/// BIC = -2 log(L(theta)) + |theta| ln(n)
/// where:
///     nonmem's OFV = -2 log(L(theta)) (log likelihood)
///     |theta| is length of estimated (non-fixed) parameters
///     n: number of observations
pub fn bic(ofv: f64, n_params: usize, n_obs: usize) -> f64 {
    let k = n_params as f64;
    let n = n_obs as f64;
    ofv + k * n.ln()
}
