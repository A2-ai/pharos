use extendr_api::prelude::*;
use extendr_api::scalar::Rfloat;

use std::str::FromStr;

use nonmem::output_files::ext::ParameterType;
use nonmem::transforms::Transform;

use hyperion_core::extendr_err;

const TRANSFORM_FROM_STR_ERR: &str = "Could not determine Transformation Type. Viable options are LogNormal, AddErr, Proportional, Identity";
const PARAMTYPE_FROM_STR_ERR: &str =
    "Could not determine ParameterType. Viable options are Theta, Omega, Sigma";

/// Parse Strings into Vec<Transform>, erroring on invalid or NA values
/// If `target_len` is provided and transforms has length 1, recycles to that length
fn parse_transforms(transforms: &Strings, target_len: usize) -> Result<Vec<Transform>> {
    let parsed: Result<Vec<Transform>> = transforms
        .iter()
        .enumerate()
        .map(|(i, t)| {
            if t.is_na() {
                Err(extendr_err!("NA transform at index {}", i + 1))
            } else {
                Transform::from_str(t.as_ref()).map_err(|_| {
                    extendr_err!(
                        "Invalid transform '{}' at index {}: {}",
                        t.as_ref(),
                        i + 1,
                        TRANSFORM_FROM_STR_ERR
                    )
                })
            }
        })
        .collect();

    let parsed = parsed?;

    // Recycle single transform to target length if needed
    if parsed.len() == 1 && target_len > 1 {
        return Ok(vec![parsed[0].clone(); target_len]);
    }

    Ok(parsed)
}

/// Parse Strings into Vec<ParameterType>, erroring on invalid or NA values
fn parse_param_types(param_types: &Strings) -> Result<Vec<ParameterType>> {
    param_types
        .iter()
        .enumerate()
        .map(|(i, pt)| {
            if pt.is_na() {
                Err(extendr_err!("NA param_type at index {}", i + 1))
            } else {
                ParameterType::from_str(pt.as_ref()).map_err(|_| {
                    extendr_err!(
                        "Invalid param_type '{}' at index {}: {}",
                        pt.as_ref(),
                        i + 1,
                        PARAMTYPE_FROM_STR_ERR
                    )
                })
            }
        })
        .collect()
}

/// Compute coefficient of variation (CV%) for random effect parameters
///
/// Calculates the CV% for Omega/Sigma diagonal parameters based on the
/// specified transformation. For LogNormal and AddErr transforms, uses
/// `sqrt(exp(estimate) - 1) * 100`. For Proportional, uses `sqrt(estimate) * 100`.
/// Returns NA for Theta parameters or Identity transform as CV is not meaningful.
///
/// @param estimate The parameter estimate(s) (variance scale), can be a vector
/// @param param_type Parameter type(s), can be a vector: "Theta", "Omega", or "Sigma"
/// @param transform Transformation type(s), can be a vector: "LogNormal", "AddErr", "Proportional", or "Identity". Defaults to "Identity".
///
/// @return CV as a percentage (vector), or NA if not applicable
/// @export
///
/// @examples \dontrun{
/// compute_cv(0.09, "Omega", "LogNormal")
/// df %>% mutate(cv = compute_cv(estimate, kind, "LogNormal"))
/// }
#[extendr]
pub fn compute_cv(
    estimate: Doubles,
    param_type: Strings,
    #[extendr(default = "identity")] transform: Strings,
) -> Result<Doubles> {
    let transforms = parse_transforms(&transform, estimate.len())?;
    let param_types = parse_param_types(&param_type)?;

    let result: Doubles = estimate
        .iter()
        .zip(param_types.iter())
        .zip(transforms.iter())
        .map(|((e, pt), tr)| {
            if e.is_na() {
                Rfloat::na()
            } else {
                tr.compute_cv(e.0, pt).map_or(Rfloat::na(), Rfloat::from)
            }
        })
        .collect();
    Ok(result)
}

/// Compute confidence interval for parameter estimates
///
/// Calculates confidence intervals using the Wald method with optional
/// back-transformation. For LogNormal transform, the CI is computed on the
/// log scale and then exponentiated. For other transforms, standard
/// symmetric intervals are computed.
///
/// @param estimate The parameter estimate(s), can be a vector
/// @param se The standard error(s) of the estimate(s), can be a vector
/// @param ci_level Confidence level between 0 and 1 (e.g., 0.95 for 95% CI). Defaults to 0.95.
/// @param transform Transformation type(s), can be a vector: "LogNormal", "AddErr", "Proportional", or "Identity". Defaults to "Identity".
///
/// @return A list with `lower` and `upper` vectors for the CI bounds
/// @export
///
/// @examples \dontrun{
/// compute_ci(1.5, 0.2)$lower
/// df %>% mutate(
///   ci_lower = compute_ci(estimate, se)$lower,
///   ci_upper = compute_ci(estimate, se)$upper
/// )
/// }
#[extendr]
pub fn compute_ci(
    estimate: Doubles,
    se: Doubles,
    #[extendr(default = "0.95")] ci_level: f64,
    #[extendr(default = "identity")] transform: Strings,
) -> Result<Robj> {
    let transforms = parse_transforms(&transform, estimate.len())?;

    let (lower_vec, upper_vec): (Vec<Rfloat>, Vec<Rfloat>) = estimate
        .iter()
        .zip(se.iter())
        .zip(transforms.iter())
        .map(|((est, stderr), tr)| {
            if est.is_na() || stderr.is_na() {
                (Rfloat::na(), Rfloat::na())
            } else {
                match tr.compute_ci(est.0, stderr.0, ci_level) {
                    Ok(ci) => (Rfloat::from(ci.0), Rfloat::from(ci.1)),
                    Err(_) => (Rfloat::na(), Rfloat::na()),
                }
            }
        })
        .unzip();

    let lower: Doubles = lower_vec.into_iter().collect();
    let upper: Doubles = upper_vec.into_iter().collect();

    let ci_list = vec![("lower", lower.into_robj()), ("upper", upper.into_robj())];

    let list = List::from_pairs(ci_list);
    Ok(list.into_robj())
}

/// Compute relative standard error (RSE%) for parameter estimates
///
/// Calculates the RSE% based on the specified transformation and parameter type.
/// For LogNormal Omega/Sigma, uses `sqrt(exp(se^2) - 1) * 100`.
/// For other transforms, uses `se / |estimate| * 100`.
///
/// @param estimate The parameter estimate(s), can be a vector
/// @param se The standard error(s) of the estimate(s), can be a vector
/// @param param_type Parameter type(s), can be a vector: "Theta", "Omega", or "Sigma"
/// @param transform Transformation type(s), can be a vector: "LogNormal", "AddErr", "Proportional", or "Identity". Defaults to "Identity".
///
/// @return RSE as a percentage (vector)
/// @export
///
/// @examples \dontrun{
/// compute_rse(1.5, 0.2, "Theta")
/// df %>% mutate(rse = compute_rse(estimate, stderr, kind))
/// }
#[extendr]
pub fn compute_rse(
    estimate: Doubles,
    se: Doubles,
    param_type: Strings,
    #[extendr(default = "identity")] transform: Strings,
) -> Result<Doubles> {
    let transforms = parse_transforms(&transform, estimate.len())?;
    let param_types = parse_param_types(&param_type)?;

    let result: Doubles = estimate
        .iter()
        .zip(se.iter())
        .zip(param_types.iter())
        .zip(transforms.iter())
        .map(|(((est, stderr), pt), tr)| {
            if stderr.is_na() {
                Rfloat::na()
            } else {
                Rfloat::from(tr.compute_rse(est.0, stderr.0, pt))
            }
        })
        .collect();

    Ok(result)
}

/// Back-transform a parameter value to the natural scale
///
/// Applies the inverse transformation to convert a parameter from the
/// estimation scale to the natural/interpretable scale. For LogNormal,
/// this exponentiates the value. For Identity, Proportional, and AddErr,
/// the value is returned unchanged.
///
/// @param value The parameter value(s) on the estimation scale, can be a vector
/// @param transform Transformation type(s), can be a vector: "LogNormal", "AddErr", "Proportional", or "Identity"
///
/// @return The back-transformed value(s) on the natural scale
/// @export
///
/// @examples \dontrun{
/// transform_value(0.5, "LogNormal")
/// transform_value(c(0.5, 1.0), "LogNormal")
/// }
#[extendr]
pub fn transform_value(value: Doubles, transform: Strings) -> Result<Doubles> {
    let transforms = parse_transforms(&transform, value.len())?;

    let result: Doubles = value
        .iter()
        .zip(transforms.iter())
        .map(|(v, tr)| {
            if v.is_na() {
                Rfloat::na()
            } else {
                Rfloat::from(tr.back_transform(v.0))
            }
        })
        .collect();
    Ok(result)
}

extendr_module! {
    mod transforms;

    fn compute_cv;
    fn compute_ci;
    fn compute_rse;
    fn transform_value;
}
