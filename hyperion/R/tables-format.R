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

#' Merge CI columns into single bracketed format
#'
#' @param table A gt table object
#' @param ci_low Name of the lower CI column
#' @param ci_high Name of the upper CI column
#' @return gt table with CI columns merged
#' @noRd
merge_ci_columns <- function(table, ci_low = "ci_low", ci_high = "ci_high") {
  table |>
    gt::cols_merge(
      columns = c(ci_low, ci_high),
      pattern = "[{1}, {2}]",
      rows = !is.na(.data[[ci_low]]) & !is.na(.data[[ci_high]])
    )
}

#' Apply CI merge across model indices
#' @noRd
apply_comparison_ci_merge <- function(
  table,
  comparison,
  spec,
  model_indices
) {
  if (is.null(spec) || !all(c("ci_low", "ci_high") %in% spec@columns)) {
    return(table)
  }
  for (idx in model_indices) {
    ci_low <- paste0("ci_low_", idx)
    ci_high <- paste0("ci_high_", idx)
    if (all(c(ci_low, ci_high) %in% names(comparison))) {
      table <- merge_ci_columns(table, ci_low, ci_high)
    }
  }
  table
}

#' Get CI percent from spec
#' @noRd
get_ci_pct <- function(spec, default = 95) {
  if (!is.null(spec) && "ci_level" %in% names(S7::props(spec))) {
    return(round(spec@ci_level * 100))
  }
  default
}

#' Apply table title when available
#' @noRd
apply_table_title <- function(table, title) {
  if (!is.null(title) && nchar(title) > 0) {
    return(table |> gt::tab_header(title = title))
  }
  table
}

#' Apply common gt formatting (labels, markdown, numeric format, missing)
#' @noRd
apply_standard_gt_formatting <- function(
  table,
  label_map,
  n_sigfig,
  numeric_cols
) {
  table |>
    gt::cols_label(!!!label_map) |>
    gt::fmt_markdown() |>
    gt::fmt_number(
      columns = dplyr::any_of(numeric_cols),
      n_sigfig = n_sigfig
    ) |>
    apply_gt_missing_text()
}

#' Check if a fixed flag is TRUE (handles logical or character)
#' @noRd
is_fixed_true <- function(x) {
  if (is.logical(x)) {
    return(!is.na(x) & x)
  }
  if (is.numeric(x)) {
    return(!is.na(x) & x != 0)
  }
  x_chr <- toupper(trimws(as.character(x)))
  !is.na(x_chr) & x_chr %in% c("TRUE", "T", "1", "YES", "Y")
}

#' Add display-friendly fixed_fmt columns
#' @noRd
add_fixed_display_columns <- function(df, fixed_cols) {
  if (length(fixed_cols) == 0) {
    return(df)
  }

  for (fc in fixed_cols) {
    if (!fc %in% names(df)) {
      next
    }
    fmt_col <- if (fc == "fixed") {
      "fixed_fmt"
    } else {
      sub("^fixed_", "fixed_fmt_", fc)
    }
    df[[fmt_col]] <- ifelse(is_fixed_true(df[[fc]]), "Fixed", "")
  }

  df
}

#' Blank CI values for fixed parameters
#' @noRd
blank_ci_for_fixed <- function(df) {
  blank_ci_cols <- function(data, fixed_col, ci_low, ci_high) {
    if (!fixed_col %in% names(data)) {
      return(data)
    }
    fixed_true <- is_fixed_true(data[[fixed_col]])
    if (any(fixed_true)) {
      if (ci_low %in% names(data)) {
        data[[ci_low]][fixed_true] <- NA_real_
      }
      if (ci_high %in% names(data)) {
        data[[ci_high]][fixed_true] <- NA_real_
      }
    }
    data
  }

  df <- blank_ci_cols(df, "fixed", "ci_low", "ci_high")

  fixed_cols <- grep("^fixed_\\d+$", names(df), value = TRUE)
  for (fc in fixed_cols) {
    idx <- sub("^fixed_", "", fc)
    df <- blank_ci_cols(
      df,
      fc,
      paste0("ci_low_", idx),
      paste0("ci_high_", idx)
    )
  }

  df
}

#' Apply missing CI formatting, keeping fixed rows empty
#' @noRd
apply_ci_missing_text <- function(
  table,
  ci_cols,
  fixed_col = NULL,
  fixed_rows = NULL
) {
  if (length(ci_cols) == 0) {
    return(table)
  }
  if (!is.null(fixed_rows)) {
    return(gt::sub_missing(
      table,
      columns = dplyr::all_of(ci_cols),
      rows = fixed_rows,
      missing_text = "-"
    ))
  }
  if (!is.null(fixed_col)) {
    rows <- rlang::expr(!is_fixed_true(.data[[!!fixed_col]]))
    return(gt::sub_missing(
      table,
      columns = dplyr::all_of(ci_cols),
      rows = !!rows,
      missing_text = "-"
    ))
  }
  gt::sub_missing(
    table,
    columns = dplyr::all_of(ci_cols),
    missing_text = "-"
  )
}

#' Format fixed columns for display
#' @noRd
apply_fixed_display <- function(table, fixed_cols) {
  for (fc in fixed_cols) {
    table <- table |>
      gt::text_transform(
        fn = function(x) ifelse(is_fixed_true(x), "Fixed", ""),
        locations = gt::cells_body(columns = fc)
      )
  }
  table
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

#' Apply standard header styling and table CSS
#' @noRd
apply_standard_gt_styling <- function(
  table,
  include_title = FALSE,
  include_row_groups = FALSE,
  include_spanners = FALSE
) {
  table <- apply_gt_bold_headers(
    table,
    include_title = include_title,
    include_row_groups = include_row_groups,
    include_spanners = include_spanners
  )
  table |>
    gt::opt_css(css = "td, th { white-space: nowrap; }")
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

#' Apply CI label overrides based on requested columns
#' @noRd
adjust_ci_labels <- function(label_map, spec, ci_pct) {
  if (is.null(spec)) {
    return(label_map)
  }

  # Get effective columns (requested minus dropped)
  dropped <- expand_ci_drop_columns(spec@drop_columns)
  effective_cols <- setdiff(spec@columns, dropped)

  ci_low_shown <- "ci_low" %in% effective_cols
  ci_high_shown <- "ci_high" %in% effective_cols

  if (ci_low_shown && !ci_high_shown) {
    label_map$ci_low <- sprintf("Lower %d%% CI", ci_pct)
  }
  if (ci_high_shown && !ci_low_shown) {
    label_map$ci_high <- sprintf("Upper %d%% CI", ci_pct)
  }
  label_map
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

  # Check for any CI columns (ci_low, ci_high, ci_low_1, ci_high_2, etc.)
  ci_cols <- grep("^ci_(low|high)", col_names, value = TRUE)
  has_ci <- length(ci_cols) > 0 &&
    any(vapply(ci_cols, function(col) any(!is.na(params[[col]])), logical(1)))

  # Check for RSE columns (handle both regular and comparison table column names)
  has_rse_regular <- "rse" %in% col_names && any(!is.na(params$rse))
  has_rse_comparison <- ("rse_1" %in% col_names && any(!is.na(params$rse_1))) ||
    ("rse_2" %in% col_names && any(!is.na(params$rse_2)))

  list(
    # Column presence
    has_ci = has_ci,
    has_rse = has_rse_regular || has_rse_comparison,
    has_stderr = "stderr" %in% col_names && any(!is.na(params$stderr)),
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

#' Build equations footnote content
#'
#' Generates equation footnotes (CI formula, % Change, CV formulas) based on
#' what statistics are present in the table.
#'
#' @param stats Named list from detect_table_statistics()
#' @param ci_pct Confidence interval percentage (e.g., 95)
#' @param comparison_stats Optional list with has_pct_change for comparison tables
#' @param summary_stats Optional list with dofv_excluded for summary tables
#' @return List of gt::md() objects for footnotes, or NULL if none
#' @noRd
build_equations_footnote <- function(
  stats,
  ci_pct,
  comparison_stats = NULL,
  summary_stats = NULL
) {
  footnotes <- list()

  # CI formula
  if (stats$has_ci) {
    footnotes <- c(
      footnotes,
      list(
        gt::md(sprintf(
          "%d%% CI: $\\mathrm{Estimate} \\pm z_{%.3g} \\cdot \\mathrm{SE}$",
          ci_pct,
          (1 - ci_pct / 100) / 2
        ))
      )
    )
  }

  # % Change formula for comparison tables
  if (!is.null(comparison_stats) && isTRUE(comparison_stats$has_pct_change)) {
    footnotes <- c(
      footnotes,
      list(
        gt::md(
          "% Change: $\\frac{\\mathrm{Estimate}_{\\mathrm{model}} - \\mathrm{Estimate}_{\\mathrm{ref}}}{\\mathrm{Estimate}_{\\mathrm{ref}}} \\cdot 100$"
        )
      )
    )
  }

  # CV formulas - group by formula type to avoid duplication

  # Formula: sqrt(exp(Est^2) - 1) * 100 (Theta LogAddErr)
  if (stats$has_theta_logadderr_cv) {
    footnotes <- c(
      footnotes,
      list(
        gt::md(
          paste0(
            "CV% for log-additive error $\\theta$: ",
            "$\\sqrt{\\exp(\\mathrm{Estimate}^2) - 1} \\times 100$"
          )
        )
      )
    )
  }

  # Formula: sqrt(exp(Est) - 1) * 100 (Omega LogNormal, Sigma LogNormal/LogAddErr)
  if (stats$has_omega_lognormal_cv || stats$has_sigma_lognormal_cv) {
    parts <- character(0)
    if (stats$has_omega_lognormal_cv) parts <- c(parts, "log-normal $\\Omega$")
    if (stats$has_sigma_lognormal_cv) parts <- c(parts, "log-normal $\\Sigma$")
    footnotes <- c(
      footnotes,
      list(
        gt::md(
          sprintf(
            "CV%% for %s: $\\sqrt{\\exp(\\mathrm{Estimate}) - 1} \\times 100$",
            paste(parts, collapse = " and ")
          )
        )
      )
    )
  }

  # Formula: sqrt(Est) * 100 (Omega Proportional, Sigma Proportional)
  if (stats$has_omega_proportional_cv || stats$has_sigma_proportional_cv) {
    parts <- character(0)
    if (stats$has_omega_proportional_cv) parts <- c(parts, "$\\Omega$")
    if (stats$has_sigma_proportional_cv) parts <- c(parts, "$\\Sigma$")
    footnotes <- c(
      footnotes,
      list(
        gt::md(
          sprintf(
            "CV%% for proportional %s: $\\sqrt{\\mathrm{Estimate}} \\times 100$",
            paste(parts, collapse = " and ")
          )
        )
      )
    )
  }

  if (length(footnotes) == 0) {
    return(NULL)
  }

  footnotes
}

#' Build abbreviations footnote content
#'
#' Generates the abbreviations section for table footnotes based on
#' what statistics are present in the table.
#'
#' @param stats Named list from detect_table_statistics()
#' @param comparison_stats Optional list with has_ofv and has_lrt for comparison tables
#' @param summary_stats Optional list with has_ofv, has_dofv, has_cond_num for summary tables
#' @return Character vector with "Abbreviations:" header + wrapped lines, or NULL
#' @noRd
build_abbreviations_footnote <- function(
  stats,
  comparison_stats = NULL,
  summary_stats = NULL
) {
  abbrevs <- character(0)
  if (stats$has_ci) abbrevs <- c(abbrevs, "CI = confidence intervals")
  if (stats$has_rse) abbrevs <- c(abbrevs, "RSE = relative standard error")
  if (stats$has_ci || stats$has_stderr) {
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

  result <- character(0)

  if (length(abbrevs) > 0) {
    abbrev_text <- paste(abbrevs, collapse = "; ")
    wrapped_abbrevs <- strwrap(abbrev_text, width = 80)
    result <- c("Abbreviations:", wrapped_abbrevs)
  }

  # Summary table: note about excluded dOFV comparisons
  if (!is.null(summary_stats) && isTRUE(summary_stats$dofv_excluded)) {
    result <- c(
      result,
      "\u0394OFV only calculated when number of observations matches reference model"
    )
  }

  if (length(result) == 0) {
    return(NULL)
  }

  result
}

#' Add footnotes to a gt table in specified order
#'
#' Coordinator function that applies footnotes from builders in the order
#' specified by spec@footnote_order.
#'
#' @param table A gt table object
#' @param spec TableSpec or SummarySpec object (can be NULL)
#' @param summary_note Character string for summary info, or NULL
#' @param equations List of footnote content for equations, or NULL
#' @param abbreviations Character vector for abbreviations, or NULL
#' @return gt table with footnotes added
#' @noRd
add_footnotes <- function(
  table,
  spec,
  summary_note,
  equations,
  abbreviations
) {
  # Get footnote order from spec - return early if NULL (disabled)
  footnote_order <- if (
    !is.null(spec) && "footnote_order" %in% names(S7::props(spec))
  ) {
    spec@footnote_order
  }
  if (is.null(footnote_order)) {
    return(table)
  }

  footnotes <- list(
    summary_info = summary_note,
    equations = equations,
    abbreviations = abbreviations
  )

  for (section in footnote_order) {
    content <- footnotes[[section]]
    if (!is.null(content)) {
      for (line in content) {
        table <- table |> gt::tab_footnote(line)
      }
    }
  }

  table
}

#' Add conditional footnotes based on table contents
#'
#' @param table A gt table object
#' @param params Parameter data frame (or comparison data frame or summary data frame)
#' @param spec TableSpec or SummarySpec object
#' @param comparison_stats Optional list with has_ofv and has_lrt for comparison tables
#' @param summary_stats Optional list with has_ofv, has_dofv, has_cond_num for summary tables
#' @param summary_note Optional character string for summary info footnote
#' @return gt table with appropriate footnotes added
#' @noRd
add_conditional_footnotes <- function(
  table,
  params,
  spec,
  comparison_stats = NULL,
  summary_stats = NULL,
  summary_note = NULL
) {
  stats <- detect_table_statistics(params)

  # Check if CI columns are dropped via spec
  if (!is.null(spec) && "drop_columns" %in% names(S7::props(spec))) {
    expanded_drop <- expand_ci_drop_columns(spec@drop_columns)
    if (all(c("ci_low", "ci_high") %in% expanded_drop)) {
      stats$has_ci <- FALSE
    }
  }

  ci_pct <- if (!is.null(spec) && "ci_level" %in% names(S7::props(spec))) {
    round(spec@ci_level * 100)
  } else {
    95
  }

  # Build footnote content using builder functions
  abbreviations <- build_abbreviations_footnote(
    stats,
    comparison_stats,
    summary_stats
  )
  equations <- build_equations_footnote(
    stats,
    ci_pct,
    comparison_stats,
    summary_stats
  )

  # Add footnotes in specified order
  add_footnotes(table, spec, summary_note, equations, abbreviations)
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
