# ==============================================================================
# Model comparison functions
# ==============================================================================

#' Compare two enriched parameter data frames
#'
#' Joins two enriched parameter data frames for side-by-side comparison.
#' Both inputs should be prepared using the standard pipeline:
#' `get_parameters() |> apply_table_spec() |> add_summary_info()`.
#'
#' @param params1 Enriched parameter data frame from model 1
#' @param params2 Enriched parameter data frame from model 2
#' @param labels Character vector of length 2 for model labels in table headers.
#'   Default: c("Model 1", "Model 2")
#'
#' @return Data frame with class `hyperion_comparison` containing joined
#'   parameter data with suffixed columns and comparison attributes.
#'
#' @export
compare_with <- function(params1, params2, labels = c("Model 1", "Model 2")) {
  if (!requireNamespace("dplyr", quietly = TRUE)) {
    stop("Package 'dplyr' is required for compare_with()")
  }

  # Validate labels
  if (length(labels) != 2) {
    stop("labels must be a character vector of length 2")
  }

  # Extract attributes from both dataframes
  spec1 <- attr(params1, "table_spec")
  spec2 <- attr(params2, "table_spec")
  sum1 <- attr(params1, "model_summary")
  sum2 <- attr(params2, "model_summary")

  # Warn if missing attributes but don't fail
  if (is.null(sum1)) {
    warning(
      "params1 is missing model_summary attribute - footnote stats will be incomplete"
    )
  }
  if (is.null(sum2)) {
    warning(
      "params2 is missing model_summary attribute - footnote stats will be incomplete"
    )
  }

  # Use spec from params1 as the canonical spec
  spec <- spec1

  # Columns to suffix (model-specific values)
  suffix_cols <- c(
    "symbol",
    "unit",
    "estimate",
    "rse",
    "ci_low",
    "ci_high",
    "variability",
    "stderr",
    "fixed",
    "shrinkage"
  )

  # Columns to coalesce (should be same across models, or take first non-NA)
  coalesce_cols <- c("kind", "section", "random_effect", "diagonal")

  # Select relevant columns from each dataframe
  keep_cols <- c("name", suffix_cols, coalesce_cols)
  keep_cols1 <- intersect(keep_cols, names(params1))
  keep_cols2 <- intersect(keep_cols, names(params2))

  p1 <- params1[, keep_cols1, drop = FALSE]
  p2 <- params2[, keep_cols2, drop = FALSE]

  # Rename suffix columns with _1 and _2
  for (col in suffix_cols) {
    if (col %in% names(p1)) {
      names(p1)[names(p1) == col] <- paste0(col, "_1")
    }
    if (col %in% names(p2)) {
      names(p2)[names(p2) == col] <- paste0(col, "_2")
    }
  }

  # Full outer join by name
  comparison <- dplyr::full_join(p1, p2, by = "name", suffix = c("_1", "_2"))

  # Coalesce shared columns
  for (col in coalesce_cols) {
    col1 <- paste0(col, "_1")
    col2 <- paste0(col, "_2")
    if (col1 %in% names(comparison) && col2 %in% names(comparison)) {
      comparison[[col]] <- dplyr::coalesce(
        comparison[[col1]],
        comparison[[col2]]
      )
      comparison[[col1]] <- NULL
      comparison[[col2]] <- NULL
    }
  }

  # Calculate percent change: (est2 - est1) / est1 * 100
  if (
    "estimate_1" %in% names(comparison) && "estimate_2" %in% names(comparison)
  ) {
    comparison$pct_change <- dplyr::case_when(
      is.na(comparison$estimate_1) | is.na(comparison$estimate_2) ~ NA_real_,
      comparison$estimate_1 == 0 ~ NA_real_,
      TRUE ~
        (comparison$estimate_2 - comparison$estimate_1) /
          comparison$estimate_1 *
          100
    )
  }

  # Attach class and attributes
  class(comparison) <- c("hyperion_comparison", class(comparison))
  attr(comparison, "summary1") <- sum1
  attr(comparison, "summary2") <- sum2
  attr(comparison, "labels") <- labels
  attr(comparison, "table_spec") <- spec

  comparison
}

#' Detect which statistics are present in a comparison table
#'
#' @param comparison Data frame from compare_with()
#' @return Named list of logicals indicating which stats are present
#' @noRd
detect_comparison_statistics <- function(comparison) {
  sum1 <- attr(comparison, "summary1")
  sum2 <- attr(comparison, "summary2")

  # Check if OFV is shown
  ofv1 <- if (!is.null(sum1) && !is.null(sum1$ofv)) sum1$ofv else NA
  ofv2 <- if (!is.null(sum2) && !is.null(sum2$ofv)) sum2$ofv else NA
  has_ofv <- !is.na(ofv1) || !is.na(ofv2)

  # Check if LRT is shown (both OFVs, same nobs, df > 0)
  has_lrt <- FALSE
  if (!is.na(ofv1) && !is.na(ofv2)) {
    nobs1 <- if (!is.null(sum1) && !is.null(sum1$number_obs)) {
      sum1$number_obs
    } else {
      NA
    }
    nobs2 <- if (!is.null(sum2) && !is.null(sum2$number_obs)) {
      sum2$number_obs
    } else {
      NA
    }
    same_nobs <- !is.na(nobs1) && !is.na(nobs2) && nobs1 == nobs2

    if (same_nobs) {
      fixed1 <- comparison$fixed_1
      fixed2 <- comparison$fixed_2
      k1 <- sum(!is.na(fixed1) & !fixed1, na.rm = TRUE)
      k2 <- sum(!is.na(fixed2) & !fixed2, na.rm = TRUE)
      df <- abs(k2 - k1)
      has_lrt <- df > 0
    }
  }

  # Check if pct_change is shown
  has_pct_change <- "pct_change" %in%
    names(comparison) &&
    any(!is.na(comparison$pct_change))

  list(
    has_ofv = has_ofv,
    has_lrt = has_lrt,
    has_pct_change = has_pct_change
  )
}

#' Build comparison footnote with OFV and LRT statistics
#'
#' @param comparison Data frame from compare_with()
#' @param n_sigfig Number of significant figures for formatting
#' @return Character vector of footnote lines, or NULL if no summaries
#' @noRd
build_comparison_footnote <- function(comparison, n_sigfig) {
  sum1 <- attr(comparison, "summary1")
  sum2 <- attr(comparison, "summary2")
  labels <- attr(comparison, "labels")

  if (is.null(sum1) && is.null(sum2)) {
    return(NULL)
  }

  lines <- character(0)

  # Get condition numbers (handle NULL from show_cond_num = FALSE)
  cn1 <- if (!is.null(sum1) && !is.null(sum1$condition_number)) {
    sum1$condition_number
  } else {
    NA
  }
  cn2 <- if (!is.null(sum2) && !is.null(sum2$condition_number)) {
    sum2$condition_number
  } else {
    NA
  }

  # Line 1: Condition Number
  if (!is.na(cn1) || !is.na(cn2)) {
    cn1_str <- if (!is.na(cn1)) {
      format_hyperion_sigfig_string(cn1, n_sigfig)
    } else {
      "N/A"
    }
    cn2_str <- if (!is.na(cn2)) {
      format_hyperion_sigfig_string(cn2, n_sigfig)
    } else {
      "N/A"
    }
    lines <- c(
      lines,
      sprintf(
        "Condition Number: %s (%s), %s (%s)",
        cn1_str,
        labels[1],
        cn2_str,
        labels[2]
      )
    )
  }

  # Get number of observations (handle NULL from show_number_obs = FALSE)
  nobs1 <- if (!is.null(sum1) && !is.null(sum1$number_obs)) {
    sum1$number_obs
  } else {
    NA
  }
  nobs2 <- if (!is.null(sum2) && !is.null(sum2$number_obs)) {
    sum2$number_obs
  } else {
    NA
  }

  # Line 2: Number Observations
  if (!is.na(nobs1) || !is.na(nobs2)) {
    nobs1_str <- if (!is.na(nobs1)) as.character(nobs1) else "N/A"
    nobs2_str <- if (!is.na(nobs2)) as.character(nobs2) else "N/A"
    lines <- c(
      lines,
      sprintf(
        "No. of Observations: %s (%s), %s (%s)",
        nobs1_str,
        labels[1],
        nobs2_str,
        labels[2]
      )
    )
  }

  # Get OFVs (handle NULL from show_ofv = FALSE)
  ofv1 <- if (!is.null(sum1) && !is.null(sum1$ofv)) sum1$ofv else NA
  ofv2 <- if (!is.null(sum2) && !is.null(sum2$ofv)) sum2$ofv else NA

  # Line 3: OFV with Delta and LRT
  if (!is.na(ofv1) || !is.na(ofv2)) {
    ofv1_str <- if (!is.na(ofv1)) {
      format_hyperion_sigfig_string(ofv1, n_sigfig)
    } else {
      "-"
    }
    ofv2_str <- if (!is.na(ofv2)) {
      format_hyperion_sigfig_string(ofv2, n_sigfig)
    } else {
      "-"
    }

    ofv_parts <- c(
      sprintf("OFV: %s (%s), %s (%s)", ofv1_str, labels[1], ofv2_str, labels[2])
    )

    # Calculate delta OFV and LRT if both OFVs available
    # Calculate LRT only if same number of observations
    if (!is.na(ofv1) && !is.na(ofv2)) {
      same_nobs <- !is.na(nobs1) && !is.na(nobs2) && nobs1 == nobs2
      if (same_nobs) {
        delta_ofv <- ofv2 - ofv1
        # Count non-fixed parameters for degrees of freedom
        fixed1 <- comparison$fixed_1
        fixed2 <- comparison$fixed_2

        # Count non-fixed params in each model (NA means param doesn't exist)
        k1 <- sum(!is.na(fixed1) & !fixed1, na.rm = TRUE)
        k2 <- sum(!is.na(fixed2) & !fixed2, na.rm = TRUE)
        df <- abs(k2 - k1)

        if (df > 0) {
          p_value <- stats::pchisq(abs(delta_ofv), df, lower.tail = FALSE)
          ofv_parts <- c(
            ofv_parts,
            sprintf(
              "delta = %s, LRT p-value = %s (df=%d)",
              format_hyperion_sigfig_string(delta_ofv, n_sigfig),
              format(p_value, scientific = TRUE, digits = n_sigfig),
              df
            )
          )
        }
      }
    }

    lines <- c(lines, sprintf("%s", paste(ofv_parts, collapse = " | ")))
  }

  if (length(lines) > 0) lines else NULL
}

#' Build GT comparison table
#'
#' Creates a formatted gt table comparing parameters from two models.
#'
#' @param comparison Comparison data frame from `compare_with()`
#'
#' @importFrom rlang .data
#'
#' @return A gt table object
#' @export
make_comparison_table <- function(comparison) {
  if (!requireNamespace("dplyr", quietly = TRUE)) {
    stop("Package 'dplyr' is required for make_comparison_table()")
  }
  if (!requireNamespace("gt", quietly = TRUE)) {
    stop("Package 'gt' is required for make_comparison_table()")
  }

  if (!inherits(comparison, "hyperion_comparison")) {
    stop("Input must be a hyperion_comparison object from compare_with()")
  }

  # Preserve attributes before dplyr operations (which strip custom attrs)
  sum1 <- attr(comparison, "summary1")
  sum2 <- attr(comparison, "summary2")
  labels <- attr(comparison, "labels")
  spec <- attr(comparison, "table_spec")
  n_sigfig <- if (!is.null(spec)) spec@n_sigfig else 3

  # Order by section if sections exist
  if ("section" %in% names(comparison) && !all(is.na(comparison$section))) {
    if (!is.null(spec) && length(spec@sections) > 0) {
      section_levels <- get_section_order(spec)
      comparison <- comparison |>
        dplyr::mutate(
          .appear_order = dplyr::row_number(),
          section = factor(.data$section, levels = section_levels)
        ) |>
        dplyr::arrange(.data$section, .data$.appear_order)
    }
  }

  # Restore attributes after dplyr operations
  attr(comparison, "summary1") <- sum1
  attr(comparison, "summary2") <- sum2
  attr(comparison, "labels") <- labels

  # Columns to hide (internal)
  hide_cols <- c(
    "kind",
    "random_effect",
    "diagonal",
    ".appear_order",
    "fixed_1",
    "fixed_2",
    "stderr_1",
    "stderr_2",
    "variability_1",
    "variability_2",
    "shrinkage_1",
    "shrinkage_2"
  )
  hide_cols <- intersect(hide_cols, names(comparison))

  # Find columns that are all NA/empty (auto-hide these if enabled)
  if (!is.null(spec) && spec@hide_empty_columns) {
    empty_cols <- find_empty_columns(comparison)
    hide_cols <- unique(c(hide_cols, empty_cols))
  }

  # Apply drop_columns from spec to comparison-specific columns
  if (!is.null(spec) && length(spec@drop_columns) > 0) {
    drop_cols <- sub("_left$", "_1", spec@drop_columns)
    drop_cols <- sub("_right$", "_2", drop_cols)
    suffix_cols <- c(
      "symbol",
      "unit",
      "estimate",
      "rse",
      "ci_low",
      "ci_high",
      "variability",
      "stderr",
      "fixed",
      "shrinkage"
    )
    drop_suffix <- intersect(drop_cols, suffix_cols)
    drop_expanded <- unlist(
      lapply(
        drop_suffix,
        function(col) c(paste0(col, "_1"), paste0(col, "_2"))
      ),
      use.names = FALSE
    )
    if (
      "ci" %in% drop_cols || "ci_low" %in% drop_cols || "ci_high" %in% drop_cols
    ) {
      drop_expanded <- c(
        drop_expanded,
        "ci_low_1",
        "ci_high_1",
        "ci_low_2",
        "ci_high_2"
      )
    }
    if ("ci_left" %in% drop_cols || "ci_1" %in% drop_cols) {
      drop_expanded <- c(drop_expanded, "ci_low_1", "ci_high_1")
    }
    if ("ci_right" %in% drop_cols || "ci_2" %in% drop_cols) {
      drop_expanded <- c(drop_expanded, "ci_low_2", "ci_high_2")
    }
    drop_expanded <- c(drop_expanded, intersect(drop_cols, names(comparison)))
    hide_cols <- unique(c(hide_cols, drop_expanded))
  }

  # Determine groupname column
  groupname <- if (
    "section" %in% names(comparison) && !all(is.na(comparison$section))
  ) {
    "section"
  } else {
    NULL
  }

  # Build gt table
  table <- comparison |>
    gt::gt(groupname_col = groupname)

  ci_pct <- if (!is.null(spec)) round(spec@ci_level * 100) else 95

  # CI merge - only if columns exist and CI values are present
  if (all(c("ci_low_1", "ci_high_1", "fixed_1") %in% names(comparison))) {
    table <- table |>
      gt::cols_merge(
        columns = c("ci_low_1", "ci_high_1", "fixed_1"),
        rows = !.data$fixed_1 & !is.na(.data$ci_low_1),
        pattern = "[{1}, {2}]"
      ) |>
      gt::cols_merge(
        columns = c("ci_low_1", "ci_high_1", "fixed_1"),
        rows = .data$fixed_1 & !is.na(.data$fixed_1),
        pattern = "Fixed"
      )
  }
  if (all(c("ci_low_2", "ci_high_2", "fixed_2") %in% names(comparison))) {
    table <- table |>
      gt::cols_merge(
        columns = c("ci_low_2", "ci_high_2", "fixed_2"),
        rows = !.data$fixed_2 & !is.na(.data$ci_low_2),
        pattern = "[{1}, {2}]"
      ) |>
      gt::cols_merge(
        columns = c("ci_low_2", "ci_high_2", "fixed_2"),
        rows = .data$fixed_2 & !is.na(.data$fixed_2),
        pattern = "Fixed"
      )
  }

  # Hide internal columns
  if (length(hide_cols) > 0) {
    table <- table |>
      gt::cols_hide(dplyr::all_of(hide_cols))
  }

  # Create spanners for each model
  # Dynamically find model columns based on suffix, in dataframe column order
  all_cols <- names(comparison)
  model1_cols <- all_cols[grepl("_1$", all_cols)]
  model2_cols <- all_cols[grepl("_2$", all_cols)]

  # Remove hidden columns (preserve order)
  model1_cols <- model1_cols[!model1_cols %in% hide_cols]
  model2_cols <- model2_cols[!model2_cols %in% hide_cols]

  # ci_high gets merged into ci_low, so remove from visible column lists
  if ("ci_low_1" %in% model1_cols) {
    model1_cols <- model1_cols[model1_cols != "ci_high_1"]
  }
  if ("ci_low_2" %in% model2_cols) {
    model2_cols <- model2_cols[model2_cols != "ci_high_2"]
  }

  if (length(model1_cols) > 0) {
    table <- table |>
      gt::tab_spanner(label = labels[1], columns = dplyr::all_of(model1_cols))
  }
  if (length(model2_cols) > 0) {
    table <- table |>
      gt::tab_spanner(label = labels[2], columns = dplyr::all_of(model2_cols))
  }

  # Rename columns for display
  label_map <- list(
    name = "Parameter",
    symbol_1 = "Symbol",
    unit_1 = "Unit",
    estimate_1 = "Estimate",
    ci_low_1 = sprintf("%d%% CI", ci_pct),
    rse_1 = "RSE (%)",
    symbol_2 = "Symbol",
    unit_2 = "Unit",
    estimate_2 = "Estimate",
    ci_low_2 = sprintf("%d%% CI", ci_pct),
    rse_2 = "RSE (%)",
    pct_change = "% Change"
  )
  label_map <- label_map[setdiff(
    intersect(names(label_map), names(comparison)),
    hide_cols
  )]

  table <- table |>
    gt::cols_label(!!!label_map) |>
    gt::fmt_markdown() |>
    gt::fmt_number(
      columns = dplyr::any_of(c(
        "estimate_1",
        "estimate_2",
        "rse_1",
        "rse_2",
        "ci_low_1",
        "ci_high_1",
        "ci_low_2",
        "ci_high_2",
        "pct_change"
      )),
      n_sigfig = n_sigfig
    ) |>
    gt::sub_missing(columns = dplyr::everything(), missing_text = "")

  # Add title if spec has one
  if (!is.null(spec) && !is.null(spec@title) && nchar(spec@title) > 0) {
    table <- table |>
      gt::tab_header(title = paste("Comparison:", spec@title))
  }

  # Add comparison footnotes (each line is a separate footnote)
  footnote_lines <- build_comparison_footnote(comparison, n_sigfig)
  if (!is.null(footnote_lines)) {
    for (fn_line in footnote_lines) {
      table <- table |>
        gt::tab_footnote(fn_line)
    }
  }

  # Compute comparison stats for conditional footnotes
  comparison_stats <- detect_comparison_statistics(comparison)

  # Add conditional footnotes (CI formula, abbreviations)
  table <- add_conditional_footnotes(table, comparison, spec, comparison_stats)

  # Style: bold headers
  table <- table |>
    gt::tab_style(
      style = gt::cell_text(weight = "bold"),
      locations = list(
        gt::cells_column_labels(dplyr::everything()),
        gt::cells_column_spanners(dplyr::everything()),
        gt::cells_row_groups()
      )
    )

  # Add vertical borders between model sections
  border_cols <- c(
    utils::tail(model1_cols, 1),
    if ("pct_change" %in% names(comparison)) utils::tail(model2_cols, 1)
  )

  if (length(border_cols) > 0) {
    table <- table |>
      gt::tab_style(
        style = gt::cell_borders(sides = "right", color = "#D3D3D3"),
        locations = gt::cells_body(columns = dplyr::all_of(border_cols))
      )
  }

  table <- table |>
    gt::opt_css(css = "td, th { white-space: nowrap; }")

  table
}
