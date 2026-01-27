#' @noRd
theta_fields <- function() {
  c("name", "display", "description", "unit", "parameterization")
}

#' @noRd
omega_fields <- function() {
  c(
    "name",
    "display",
    "description",
    "parameterization",
    "associated_theta"
  )
}

#' @noRd
sigma_fields <- function() {
  c("name", "display", "description", "unit", "parameterization")
}

#' @noRd
valid_parameterizations <- function() {
  c(
    "LogNormal",
    "Logit",
    "AddErr",
    "LogAddErr",
    "Proportional",
    "Identity"
  )
}
