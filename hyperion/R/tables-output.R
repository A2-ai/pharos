#' Build summary footnote from model summary
#'
#' @param params Data frame with model_summary attribute
#' @param n_sigfig Number of significant figures for formatting
#' @return Character string for footnote, or NULL if no summary
#' @noRd
build_summary_footnote <- function(params, n_sigfig) {
  model_sum <- attr(params, "model_summary")
  if (is.null(model_sum)) {
    return(NULL)
  }

  parts <- character(0)

  if (
    !is.null(model_sum$estimation_method) &&
      !is.na(model_sum$estimation_method)
  ) {
    parts <- c(parts, model_sum$estimation_method)
  }

  if (!is.null(model_sum$ofv) && !is.na(model_sum$ofv)) {
    parts <- c(
      parts,
      sprintf(
        "Objective function value: %s",
        format_hyperion_sigfig_string(model_sum$ofv, n_sigfig)
      )
    )
  }

  if (
    !is.null(model_sum$condition_number) &&
      !is.na(model_sum$condition_number)
  ) {
    parts <- c(
      parts,
      sprintf(
        "Condition Number: %s",
        format_hyperion_sigfig_string(model_sum$condition_number, n_sigfig)
      )
    )
  }

  if (length(parts) > 0) paste(parts, collapse = " | ") else NULL
}

#' Add model summary information for table footnote
#'
#' Attaches estimation method, OFV, and condition number to parameter data
#' for display as the first footnote in the parameter table.
#'
#' @param params Enriched parameter data frame from `apply_table_spec()`
#' @param sum Summary object from `get_model_summary()`, or NULL to skip
#'
#' @return Data frame with model_summary attribute attached
#' @export
add_summary_info <- function(params, sum) {
  if (is.null(sum)) {
    return(params)
  }

  attr(params, "model_summary") <- list(
    estimation_method = dplyr::last(sum$run_details$estimation_method),
    ofv = dplyr::last(sum$minimization_results$ofv),
    condition_number = dplyr::last(sum$minimization_results$condition_number)
  )

  params
}

#' Extract TableSpec from a parameter data frame
#'
#' Retrieves the `TableSpec` attached to a parameter data frame (e.g., from
#' `apply_table_spec()`). Returns NULL if none is found.
#'
#' @param params Data frame carrying a `table_spec` attribute
#'
#' @return A TableSpec object or NULL
#' @export
get_table_spec <- function(params) {
  spec <- attr(params, "table_spec")
  if (is.null(spec)) {
    return(NULL)
  }
  if (!S7::S7_inherits(spec, TableSpec)) {
    stop("Attached table_spec is not a TableSpec object")
  }
  spec
}

#' Order sections and select columns
#'
#' Orders rows by section according to the spec, and selects the appropriate columns.
#'
#' @param params Data frame with summary rows from `add_summary_rows()`
#' @param spec A TableSpec object
#'
#' @importFrom rlang .data
#'
#' @return Reordered data frame ready for `make_parameter_table()`
#' @noRd
#' @keywords internal
order_sections <- function(params, spec) {
  if (!requireNamespace("dplyr", quietly = TRUE)) {
    stop("Package 'dplyr' is required for order_sections()")
  }
  section_levels <- get_section_order(spec)

  internal_cols <- c(
    "section",
    ".appear_order",
    "kind",
    "random_effect",
    "diagonal",
    "transforms",
    "cv",
    "corr",
    "sd"
  )
  dt_cols <- grep("^dt_", names(params), value = TRUE)

  # Only include internal columns that actually exist in the data
  internal_cols <- intersect(internal_cols, names(params))

  params |>
    dplyr::mutate(
      .appear_order = dplyr::row_number(),
      section = factor(.data$section, levels = section_levels)
    ) |>
    dplyr::arrange(.data$section, .data$.appear_order) |>
    dplyr::select(dplyr::all_of(c(
      setdiff(spec@columns, spec@drop_columns),
      internal_cols,
      dt_cols
    )))
}

# ==============================================================================
# GT table building
# ==============================================================================

#' Build GT parameter table
#'
#' Creates a formatted gt table from parameter data.
#'
#' @param params Parameter data frame from `get_parameters()` or enriched via
#'   `apply_table_spec()`
#'
#' @importFrom rlang .data
#'
#' @return A gt table object
#' @export
make_parameter_table <- function(params) {
  if (!requireNamespace("dplyr", quietly = TRUE)) {
    stop("Package 'dplyr' is required for make_parameter_table()")
  }
  if (!requireNamespace("gt", quietly = TRUE)) {
    stop(
      "Package 'gt' is required for make_parameter_table(). Install it in the terminal with 'rv add gt'"
    )
  }

  # Get table_spec - required for proper formatting

  spec <- attr(params, "table_spec")
  if (is.null(spec)) {
    stop("TableSpec not found. Run apply_table_spec(params, info, spec) first.")
  }
  params <- order_sections(params, spec)

  # Find columns that are all NA/empty (auto-hide these)
  is_all_empty <- function(x) {
    if (is.character(x)) {
      all(is.na(x) | x == "")
    } else {
      all(is.na(x))
    }
  }
  empty_cols <- names(params)[vapply(params, is_all_empty, logical(1))]

  # Get columns to hide (internal + dt_* + raw variability components + empty)
  dt_cols <- grep("^dt_", names(params), value = TRUE)
  hide_cols <- c(
    ".appear_order",
    "kind",
    "random_effect",
    "diagonal",
    "transforms",
    "cv",
    "corr",
    "sd",
    "nonmem_name",
    "user_name",
    dt_cols,
    empty_cols
  )
  hide_cols <- intersect(hide_cols, names(params))

  # Build labels only for columns that exist
  ci_pct <- round(spec@ci_level * 100)
  label_map <- list(
    name = "Parameter",
    description = "",
    symbol = "",
    unit = "",
    estimate = "Estimate",
    ci_low = sprintf("%d%% CI", ci_pct),
    variability = "",
    rse = "RSE (%)",
    shrinkage = "Shrinkage (%)"
  )
  label_map <- label_map[intersect(names(label_map), names(params))]

  # Only use section grouping if sections were defined
  groupname <- if (length(spec@sections) > 0) "section" else NULL

  table <- params |>
    gt::gt(groupname_col = groupname) |>
    gt::cols_hide(dplyr::all_of(hide_cols))

  # CI merge - only if columns exist
  if (all(c("ci_low", "ci_high", "fixed") %in% names(params))) {
    table <- table |>
      gt::cols_merge(
        columns = c("ci_low", "ci_high", "fixed"),
        rows = !.data$fixed,
        pattern = "[{1}, {2}]"
      ) |>
      gt::cols_merge(
        columns = c("ci_low", "ci_high", "fixed"),
        rows = .data$fixed,
        pattern = "Fixed"
      )
  }

  n_sigfig <- spec@n_sigfig
  table <- table |>
    gt::cols_label(!!!label_map) |>
    gt::fmt_markdown() |>
    gt::fmt_number(
      columns = dplyr::any_of(c(
        "estimate",
        "ci_low",
        "ci_high",
        "rse",
        "shrinkage"
      )),
      n_sigfig = n_sigfig
    ) |>
    gt::sub_missing(columns = dplyr::everything(), missing_text = "")

  if (all(c("ci_low", "ci_high") %in% names(params))) {
    table <- table |>
      gt::sub_missing(
        columns = c("ci_low", "ci_high"),
        missing_text = "-"
      )
  }

  table <- table |>
    gt::tab_header(title = spec@title)

  # Add summary info as first footnote
  summary_note <- build_summary_footnote(params, spec@n_sigfig)
  if (!is.null(summary_note)) {
    table <- table |> gt::tab_footnote(summary_note)
  }

  # Add conditional footnotes based on what's actually in the table
  table <- add_conditional_footnotes(table, params, spec)

  table <- table |>
    gt::tab_style(
      style = gt::cell_text(weight = "bold"),
      locations = list(
        gt::cells_column_labels(dplyr::everything()),
        gt::cells_title(groups = "title"),
        gt::cells_row_groups()
      )
    )

  table <- table |>
    gt::opt_css(css = "td, th { white-space: nowrap; }")

  table
}
