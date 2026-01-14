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

#' Build comparison footnote with OFV and LRT statistics
#'
#' @param comparison Data frame from compare_with()
#' @param n_sigfig Number of significant figures for formatting
#' @return Character string for footnote, or NULL if no summaries
#' @noRd
build_comparison_footnote <- function(comparison, n_sigfig) {
  sum1 <- attr(comparison, "summary1")
  sum2 <- attr(comparison, "summary2")
  labels <- attr(comparison, "labels")

  if (is.null(sum1) && is.null(sum2)) {
    return(NULL)
  }

  parts <- character(0)

  # Get OFVs
  ofv1 <- if (!is.null(sum1)) sum1$ofv else NA
  ofv2 <- if (!is.null(sum2)) sum2$ofv else NA

  # Add OFV for each model
  if (!is.na(ofv1)) {
    parts <- c(
      parts,
      sprintf(
        "%s OFV: %s",
        labels[1],
        format_hyperion_sigfig_string(ofv1, n_sigfig)
      )
    )
  }
  if (!is.na(ofv2)) {
    parts <- c(
      parts,
      sprintf(
        "%s OFV: %s",
        labels[2],
        format_hyperion_sigfig_string(ofv2, n_sigfig)
      )
    )
  }

  # Calculate delta OFV and LRT if both OFVs available
  if (!is.na(ofv1) && !is.na(ofv2)) {
    delta_ofv <- ofv2 - ofv1
    parts <- c(
      parts,
      sprintf(
        "Delta: %s",
        format_hyperion_sigfig_string(delta_ofv, n_sigfig)
      )
    )

    # Count non-fixed parameters for degrees of freedom
    # We need to get this from the original params - use fixed columns
    fixed1 <- comparison$fixed_1
    fixed2 <- comparison$fixed_2

    # Count non-fixed params in each model (NA means param doesn't exist in that model)
    k1 <- sum(!is.na(fixed1) & !fixed1, na.rm = TRUE)
    k2 <- sum(!is.na(fixed2) & !fixed2, na.rm = TRUE)
    df <- abs(k2 - k1)

    if (df > 0) {
      # Chi-square test
      p_value <- stats::pchisq(abs(delta_ofv), df, lower.tail = FALSE)
      parts <- c(
        parts,
        sprintf(
          "LRT p-value: %s (df=%d)",
          format(p_value, scientific = TRUE, digits = n_sigfig),
          df
        )
      )
    }
  }

  if (length(parts) > 0) paste(parts, collapse = " | ") else NULL
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
    "ci_low_1",
    "ci_high_1",
    "ci_low_2",
    "ci_high_2",
    "variability_1",
    "variability_2",
    "shrinkage_1",
    "shrinkage_2"
  )
  hide_cols <- intersect(hide_cols, names(comparison))

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
    drop_expanded <- c(drop_expanded, intersect(drop_cols, names(comparison)))
    hide_cols <- unique(c(hide_cols, drop_expanded))
  }

  # Determine groupname column
  groupname <- if (
    "section" %in% names(comparison) && !all(is.na(comparison$section))
  )
    "section" else NULL

  # Build gt table
  table <- comparison |>
    gt::gt(groupname_col = groupname)

  # Hide internal columns
  if (length(hide_cols) > 0) {
    table <- table |>
      gt::cols_hide(dplyr::all_of(hide_cols))
  }

  # Create spanners for each model
  model1_cols <- c("symbol_1", "unit_1", "estimate_1", "rse_1")
  model2_cols <- c("symbol_2", "unit_2", "estimate_2", "rse_2")
  model1_cols <- setdiff(intersect(model1_cols, names(comparison)), hide_cols)
  model2_cols <- setdiff(intersect(model2_cols, names(comparison)), hide_cols)

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
    rse_1 = "RSE (%)",
    symbol_2 = "Symbol",
    unit_2 = "Unit",
    estimate_2 = "Estimate",
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
        "pct_change"
      )),
      n_sigfig = n_sigfig
    ) |>
    gt::sub_missing(columns = dplyr::everything(), missing_text = "-")

  # Add title if spec has one
  if (!is.null(spec) && !is.null(spec@title) && nchar(spec@title) > 0) {
    table <- table |>
      gt::tab_header(title = paste("Comparison:", spec@title))
  }

  # Add comparison footnote
  footnote <- build_comparison_footnote(comparison, n_sigfig)
  if (!is.null(footnote)) {
    table <- table |>
      gt::tab_footnote(footnote)
  }

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

  table
}
