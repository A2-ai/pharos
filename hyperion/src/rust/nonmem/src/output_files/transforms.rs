use extendr_api::prelude::*;
use extendr_api::scalar::Rfloat;

use std::str::FromStr;

use nonmem::transforms::Transform;

use hyperion_core::ResultExt;

const TRANSFORM_FROM_STR_ERR: &str = "Could not determine Transformation Type. Viable optoions are LogNormal, AddErr, Proportional, Identity";

/// Compute coefficient of variation (CV%) for random effect parameters
///
/// Calculates the CV% for Omega/Sigma diagonal parameters based on the
/// specified transformation. For LogNormal and AddErr transforms, uses
/// `sqrt(exp(estimate) - 1) * 100`. For Proportional, uses `sqrt(estimate) * 100`.
/// Returns NA for Identity transform as CV is not meaningful.
///
/// @param estimate The parameter estimate (variance scale)
/// @param transform Transformation type: "LogNormal", "AddErr", "Proportional", or "Identity"
///
/// @return CV as a percentage, or NA if not applicable
/// @export
///
/// @examples \dontrun{
/// compute_cv(0.09, "LogNormal")
/// compute_cv(0.04, "Proportional")
/// }
#[extendr]
pub fn compute_cv(estimate: f64, transform: String) -> Result<Rfloat> {
    let t = Transform::from_str(&transform).map_to_extendr_err(TRANSFORM_FROM_STR_ERR)?;
    let cv = t.compute_cv(estimate).map_or(Rfloat::na(), Rfloat::from);
    Ok(cv)
}

/// Compute confidence interval for a parameter estimate
///
/// Calculates a confidence interval using the Wald method with optional
/// back-transformation. For LogNormal transform, the CI is computed on the
/// log scale and then exponentiated. For other transforms, standard
/// symmetric intervals are computed.
///
/// @param estimate The parameter estimate
/// @param se The standard error of the estimate
/// @param ci_level Confidence level between 0 and 1 (e.g., 0.95 for 95% CI)
/// @param transform Transformation type: "LogNormal", "AddErr", "Proportional", or "Identity"
///
/// @return A data frame with columns `lower` and `upper` for the CI bounds
/// @export
///
/// @examples \dontrun{
/// compute_ci(1.5, 0.2, 0.95, "Identity")
/// compute_ci(0.3, 0.05, 0.95, "LogNormal")
/// }
#[extendr]
pub fn compute_ci(estimate: f64, se: f64, ci_level: f64, transform: String) -> Result<Robj> {
    let t = Transform::from_str(&transform).map_to_extendr_err(TRANSFORM_FROM_STR_ERR)?;
    let ci = t
        .compute_ci(estimate, se, ci_level)
        .map_to_extendr_err("Could not compute CI")?;

    let ci_interval = vec![
        ("lower", Rfloat::from(ci.0).into_robj()),
        ("upper", Rfloat::from(ci.1).into_robj()),
    ];

    let list = List::from_pairs(ci_interval);
    let df = data_frame!(list);
    Ok(df)
}

/// Back-transform a parameter value to the natural scale
///
/// Applies the inverse transformation to convert a parameter from the
/// estimation scale to the natural/interpretable scale. For LogNormal,
/// this exponentiates the value. For Identity, Proportional, and AddErr,
/// the value is returned unchanged.
///
/// @param value The parameter value on the estimation scale
/// @param transform Transformation type: "LogNormal", "AddErr", "Proportional", or "Identity"
///
/// @return The back-transformed value on the natural scale
/// @export
///
/// @examples \dontrun{
/// transform_value(0.5, "LogNormal")  # Returns exp(0.5) ≈ 1.649
/// transform_value(1.5, "Identity")   # Returns 1.5
/// }
#[extendr]
pub fn transform_value(value: f64, transform: String) -> Result<Rfloat> {
    let t = Transform::from_str(&transform).map_to_extendr_err(TRANSFORM_FROM_STR_ERR)?;
    let trans = t.back_transform(value);

    Ok(Rfloat::from(trans))
}

extendr_module! {
    mod transforms;

    fn compute_cv;
    fn compute_ci;
    fn transform_value;
}
