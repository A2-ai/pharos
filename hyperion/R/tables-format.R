# ==============================================================================
# Table helpers
# ==============================================================================

#' Find columns that are all NA or empty
#'
#' @param df Data frame to check
#' @return Character vector of column names that are all NA/empty
#' @noRd
find_empty_columns <- function(df) {
  is_all_empty <- function(x) {
    if (is.character(x)) {
      all(is.na(x) | x == "")
    } else {
      all(is.na(x))
    }
  }
  names(df)[vapply(df, is_all_empty, logical(1))]
}

#' Apply standard missing value formatting to gt tables
#' @noRd
apply_gt_missing_text <- function(table, missing_text = "") {
  table |>
    gt::sub_missing(columns = dplyr::everything(), missing_text = missing_text)
}

#' Apply standard bold styling to gt table headers
#' @noRd
apply_gt_bold_headers <- function(
  table,
  include_title = FALSE,
  include_row_groups = FALSE,
  include_spanners = FALSE
) {
  locations <- list(gt::cells_column_labels(dplyr::everything()))
  if (include_title) {
    locations <- c(locations, list(gt::cells_title(groups = "title")))
  }
  if (include_row_groups) {
    locations <- c(locations, list(gt::cells_row_groups()))
  }
  if (include_spanners) {
    locations <- c(
      locations,
      list(gt::cells_column_spanners(dplyr::everything()))
    )
  }

  table |>
    gt::tab_style(
      style = gt::cell_text(weight = "bold"),
      locations = locations
    )
}

# ==============================================================================
# Footnote helpers
# ==============================================================================

#' Build a label map for parameter table columns
#'
#' @param ci_pct Confidence interval percentage
#' @return Named list of labels for gt::cols_label()
#' @noRd
build_parameter_label_map <- function(ci_pct) {
  list(
    name = "Parameter",
    description = "",
    symbol = "Symbol",
    unit = "Unit",
    estimate = "Estimate",
    ci_low = sprintf("%d%% CI", ci_pct),
    ci_high = sprintf("%d%% CI", ci_pct),
    variability = "",
    rse = "RSE (%)",
    shrinkage = "Shrinkage (%)",
    fixed = "Fixed",
    stderr = "SE"
  )
}

#' Detect which statistics are used in a parameter table
#'
#' @param params Parameter data frame (after apply_table_spec or comparison)
#' @return Named list of logicals indicating which stats are present
#' @noRd
detect_table_statistics <- function(params) {
  has_cv_col <- "cv" %in% names(params)
  has_transforms <- "transforms" %in% names(params)
  col_names <- names(params)

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

  # Check for CI columns (handle both regular and comparison table column names)
  has_ci_regular <- all(c("ci_low", "ci_high") %in% col_names) &&
    any(!is.na(params$ci_low))
  has_ci_comparison <- (all(c("ci_low_1", "ci_high_1") %in% col_names) &&
    any(!is.na(params$ci_low_1))) ||
    (all(c("ci_low_2", "ci_high_2") %in% col_names) &&
      any(!is.na(params$ci_low_2)))

  # Check for RSE columns (handle both regular and comparison table column names)
  has_rse_regular <- "rse" %in% col_names && any(!is.na(params$rse))
  has_rse_comparison <- ("rse_1" %in% col_names && any(!is.na(params$rse_1))) ||
    ("rse_2" %in% col_names && any(!is.na(params$rse_2)))

  list(
    # Column presence
    has_ci = has_ci_regular || has_ci_comparison,
    has_rse = has_rse_regular || has_rse_comparison,
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
#' @param params Parameter data frame (or comparison data frame or summary data frame)
#' @param spec TableSpec or SummarySpec object
#' @param comparison_stats Optional list with has_ofv and has_lrt for comparison tables
#' @param summary_stats Optional list with has_ofv, has_dofv, has_cond_num for summary tables
#' @return gt table with appropriate footnotes added
#' @noRd
add_conditional_footnotes <- function(
  table,
  params,
  spec,
  comparison_stats = NULL,
  summary_stats = NULL
) {
  stats <- detect_table_statistics(params)
  ci_pct <- if (!is.null(spec) && "ci_level" %in% names(S7::props(spec))) {
    round(spec@ci_level * 100)
  } else {
    95
  }

  # Build abbreviation list dynamically
  abbrevs <- character(0)
  if (stats$has_ci) abbrevs <- c(abbrevs, "CI = confidence intervals")
  if (stats$has_rse) abbrevs <- c(abbrevs, "RSE = relative standard error")
  if (
    stats$has_ci ||
      ("stderr" %in% names(params) && any(!is.na(params$stderr)))
  ) {
    abbrevs <- c(abbrevs, "SE = standard error")
  }
  if (stats$has_cv) abbrevs <- c(abbrevs, "CV = coefficient of variation")
  if (stats$has_sd) abbrevs <- c(abbrevs, "SD = standard deviation")
  if (stats$has_corr) abbrevs <- c(abbrevs, "Corr = correlation")

  # Comparison table abbreviations
  if (!is.null(comparison_stats)) {
    if (isTRUE(comparison_stats$has_ofv)) {
      abbrevs <- c(abbrevs, "OFV = Objective Function Value")
    }
    if (isTRUE(comparison_stats$has_lrt)) {
      abbrevs <- c(abbrevs, "LRT = Likelihood Ratio Test")
      abbrevs <- c(abbrevs, "df = degrees of freedom")
    }
  }

  # Summary table abbreviations
  if (!is.null(summary_stats)) {
    if (isTRUE(summary_stats$has_ofv)) {
      abbrevs <- c(abbrevs, "OFV = Objective Function Value")
    }
    if (isTRUE(summary_stats$has_dofv)) {
      abbrevs <- c(abbrevs, "\u0394OFV = change in OFV from reference model")
    }
    if (isTRUE(summary_stats$has_cond_num)) {
      abbrevs <- c(abbrevs, "Cond. No. = Condition Number")
    }
    if (isTRUE(summary_stats$has_pvalue)) {
      abbrevs <- c(abbrevs, "p-value from LRT (Likelihood Ratio Test)")
      abbrevs <- c(abbrevs, "df = degrees of freedom")
    }
  }

  # Add abbreviations footnote if any exist
  if (length(abbrevs) > 0) {
    abbrev_text <- paste(abbrevs, collapse = "; ")
    wrapped_abbrevs <- strwrap(abbrev_text, width = 80)
    table <- table |>
      gt::tab_footnote("Abbreviations:")
    for (line in wrapped_abbrevs) {
      table <- table |> gt::tab_footnote(line)
    }
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

  # Add % Change formula for comparison tables
  if (!is.null(comparison_stats) && isTRUE(comparison_stats$has_pct_change)) {
    table <- table |>
      gt::tab_footnote(
        footnote = gt::md(
          "% Change: $\\frac{\\mathrm{Estimate}_2 - \\mathrm{Estimate}_1}{\\mathrm{Estimate}_1} \\cdot 100$"
        )
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

  # Summary table: note about excluded dOFV comparisons
  if (!is.null(summary_stats) && isTRUE(summary_stats$dofv_excluded)) {
    table <- table |>
      gt::tab_footnote(
        "\u0394OFV only calculated when number of observations matches reference model"
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
    out[is_omega] <- sprintf("\\Omega_{(%s)}", idx_str)
  }

  # SIGMA: EPS... -> Sigma
  is_sigma <- !is.na(kind) & kind == "SIGMA" & !is.na(random_effect)
  if (any(is_sigma)) {
    idx_str <- make_cov_idx(random_effect[is_sigma])
    out[is_sigma] <- sprintf("\\Sigma_{(%s)}", idx_str)
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
