use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct InformationCriteria {
    pub ofv: f64,
    pub n_estimated_parameters: usize,
    pub n_observations: usize,
    pub aic: f64,
    pub bic: f64,
}

impl InformationCriteria {
    pub fn new(ofv: f64, n_estimated_parameters: usize, n_observations: usize) -> Self {
        Self {
            ofv,
            n_estimated_parameters,
            n_observations,
            aic: aic(ofv, n_estimated_parameters, 2.0),
            bic: bic(ofv, n_estimated_parameters, n_observations),
        }
    }

    /// Recompute AIC with a non-default penalty `k`, mirroring R's `AIC(object, k = ...)`.
    /// The default constructed via `new` uses `k = 2`.
    pub fn with_penalty(mut self, penalty: f64) -> Self {
        self.aic = aic(self.ofv, self.n_estimated_parameters, penalty);
        self
    }
}

/// AIC = -2 log(L(theta)) + k|theta|
/// where:
///     nonmem's OFV = -2 log(L(theta)) (log likelihood)
///     |theta| is length of estimated (non-fixed) parameters
///     k: penalty per estimated parameter (2 for the standard AIC)
pub fn aic(ofv: f64, n_estimated_parameters: usize, penalty: f64) -> f64 {
    let df = n_estimated_parameters as f64;
    ofv + penalty * df
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
