# ==============================================================================
# Footnote helpers
# ==============================================================================

#' Detect which statistics are used in a parameter table
#'
#' @param params Parameter data frame (after apply_table_spec)
#' @return Named list of logicals indicating which stats are present
#' @noRd
detect_table_statistics <- function(params) {
  has_cv_col <- "cv" %in% names(params)
  has_transforms <- "transforms" %in% names(params)

  # Helper to check for CV with specific kind and transform
  cv_with <- function(kind, transforms) {
    has_cv_col &&
      has_transforms &&
      any(
        !is.na(params$cv) &
          params$kind == kind &
          tolower(params$transforms) %in% tolower(transforms)
      )
  }

  list(
    # Column presence
    has_ci = all(c("ci_low", "ci_high") %in% names(params)) &&
      any(!is.na(params$ci_low)),
    has_rse = "rse" %in% names(params) && any(!is.na(params$rse)),
    has_shrinkage = "shrinkage" %in%
      names(params) &&
      any(!is.na(params$shrinkage)),

    # Merged column statistics (cv/sd/corr)
    has_cv = has_cv_col && any(!is.na(params$cv)),
    has_sd = "sd" %in%
      names(params) &&
      any(!is.na(params$sd) & is.na(params$cv) & is.na(params$corr)),
    has_corr = "corr" %in% names(params) && any(!is.na(params$corr)),

    # CV formula detection by kind and transform
    # Theta LogAddErr: sqrt(exp(Est^2) - 1) * 100
    has_theta_logadderr_cv = cv_with("THETA", "logadderr"),

    # Omega LogNormal: sqrt(exp(Est) - 1) * 100
    has_omega_lognormal_cv = cv_with("OMEGA", "lognormal"),

    # Omega Proportional: sqrt(Est) * 100
    has_omega_proportional_cv = cv_with("OMEGA", "proportional"),

    # Sigma LogNormal/LogAddErr: sqrt(exp(Est) - 1) * 100
    has_sigma_lognormal_cv = cv_with("SIGMA", c("lognormal", "logadderr")),

    # Sigma Proportional: sqrt(Est) * 100
    has_sigma_proportional_cv = cv_with("SIGMA", "proportional")
  )
}

#' Add conditional footnotes based on table contents
#'
#' @param table A gt table object
#' @param params Parameter data frame
#' @param spec TableSpec object (for ci_level)
#' @return gt table with appropriate footnotes added
#' @noRd
add_conditional_footnotes <- function(table, params, spec) {
  stats <- detect_table_statistics(params)
  ci_pct <- round(spec@ci_level * 100)

  # Build abbreviation list dynamically
  abbrevs <- character(0)
  if (stats$has_ci) abbrevs <- c(abbrevs, "CI = confidence intervals")
  if (stats$has_rse) abbrevs <- c(abbrevs, "RSE = relative standard error")
  if (stats$has_cv) abbrevs <- c(abbrevs, "CV = coefficient of variation")
  if (stats$has_sd) abbrevs <- c(abbrevs, "SD = standard deviation")
  if (stats$has_corr) abbrevs <- c(abbrevs, "Corr = correlation")

  # Add abbreviations footnote if any exist
  if (length(abbrevs) > 0) {
    table <- table |>
      gt::tab_footnote("Abbreviations:") |>
      gt::tab_footnote(paste(abbrevs, collapse = "; "))
  }

  # Add CI formula if CI columns are used
  if (stats$has_ci) {
    table <- table |>
      gt::tab_footnote(
        footnote = gt::md(sprintf(
          "%d%% CI: $\\mathrm{Estimate} \\pm z_{%.3g} \\cdot \\mathrm{SE}$",
          ci_pct,
          (1 - ci_pct / 100) / 2
        ))
      )
  }

  # CV formulas - group by formula type to avoid duplication

  # Formula: sqrt(exp(Est^2) - 1) * 100 (Theta LogAddErr)
  if (stats$has_theta_logadderr_cv) {
    table <- table |>
      gt::tab_footnote(
        gt::md(
          paste0(
            "CV% for log-additive error $\\theta$: ",
            "$\\sqrt{\\exp(\\mathrm{Estimate}^2) - 1} \\times 100$"
          )
        )
      )
  }

  # Formula: sqrt(exp(Est) - 1) * 100 (Omega LogNormal, Sigma LogNormal/LogAddErr)
  if (stats$has_omega_lognormal_cv || stats$has_sigma_lognormal_cv) {
    parts <- character(0)
    if (stats$has_omega_lognormal_cv) parts <- c(parts, "log-normal $\\Omega$")
    if (stats$has_sigma_lognormal_cv) parts <- c(parts, "log-normal $\\Sigma$")
    table <- table |>
      gt::tab_footnote(
        gt::md(
          sprintf(
            "CV%% for %s: $\\sqrt{\\exp(\\mathrm{Estimate}) - 1} \\times 100$",
            paste(parts, collapse = " and ")
          )
        )
      )
  }

  # Formula: sqrt(Est) * 100 (Omega Proportional, Sigma Proportional)
  if (stats$has_omega_proportional_cv || stats$has_sigma_proportional_cv) {
    parts <- character(0)
    if (stats$has_omega_proportional_cv) parts <- c(parts, "$\\Omega$")
    if (stats$has_sigma_proportional_cv) parts <- c(parts, "$\\Sigma$")
    table <- table |>
      gt::tab_footnote(
        gt::md(
          sprintf(
            "CV%% for proportional %s: $\\sqrt{\\mathrm{Estimate}} \\times 100$",
            paste(parts, collapse = " and ")
          )
        )
      )
  }

  table
}

# ==============================================================================
# Formatting helpers (Greek symbols, markdown)
# ==============================================================================

#' Convert parameter kind to Greek symbol in LaTeX math notation
#'
#' Returns raw LaTeX (without $..$ delimiters) for use in param_symbol_md().
#' @noRd
greek_to_latex <- function(kind, random_effect) {
  stopifnot(length(kind) == length(random_effect))

  n <- length(kind)
  out <- rep(NA_character_, n)

  # THETA: enumerate in order of appearance
  is_theta <- !is.na(kind) & kind == "THETA"
  if (any(is_theta)) {
    theta_idx <- seq_len(sum(is_theta))
    out[is_theta] <- sprintf("\\theta_{%d}", theta_idx)
  }

  # Helper: from random_effect -> "row,col" for lower triangle
  # e.g. "ETA1" -> "1,1"; "ETA1:ETA2" -> "2,1"
  make_cov_idx <- function(re) {
    nums_list <- regmatches(re, gregexpr("\\d+", re))
    vapply(
      nums_list,
      function(nums_chr) {
        if (length(nums_chr) == 0L) {
          return("")
        }
        nums <- as.integer(nums_chr)

        if (length(nums) == 1L) {
          sprintf("%d,%d", nums, nums) # ETA1 -> (1,1)
        } else {
          r <- max(nums[1:2]) # ETA1:ETA2 -> (2,1)
          c <- min(nums[1:2])
          sprintf("%d,%d", r, c)
        }
      },
      character(1)
    )
  }

  # OMEGA: ETA... -> Omega
  is_omega <- !is.na(kind) & kind == "OMEGA" & !is.na(random_effect)
  if (any(is_omega)) {
    idx_str <- make_cov_idx(random_effect[is_omega])
    out[is_omega] <- sprintf("\\omega_{(%s)}", idx_str)
  }

  # SIGMA: EPS... -> Sigma
  is_sigma <- !is.na(kind) & kind == "SIGMA" & !is.na(random_effect)
  if (any(is_sigma)) {
    idx_str <- make_cov_idx(random_effect[is_sigma])
    out[is_sigma] <- sprintf("\\sigma_{(%s)}", idx_str)
  }

  out
}

#' Build parameter symbols as LaTeX math expressions
#'
#' Wraps in exp() for LogNormal and logistic for Logit transforms.
#' Returns complete LaTeX math expressions wrapped in $..$.
#' @noRd
param_symbol_md <- function(kind, random_effect, transforms) {
  base_sym <- greek_to_latex(kind, random_effect)

  tr <- transforms
  if (is.factor(tr)) tr <- as.character(tr)

  # Build raw LaTeX expression (without $..$ delimiters)
  latex_expr <- dplyr::case_when(
    !is.na(tr) & tolower(tr) == "lognormal" ~ paste0("\\exp(", base_sym, ")"),
    !is.na(tr) & tolower(tr) == "logit" ~
      paste0("1/(1 + \\exp(-", base_sym, "))"),
    TRUE ~ base_sym
  )

  # Wrap in $..$ for inline LaTeX math (only for non-NA values)
  dplyr::if_else(
    !is.na(latex_expr),
    paste0("$", latex_expr, "$"),
    NA_character_
  )
}
