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
      pattern = "[{1}, {2}]"
    )
}

#' Build summary footnote from model summary
#'
#' @param params Data frame with model_summary attribute
#' @param n_sigfig Number of significant figures for formatting
#' @return Character string for footnote, or NULL if no summary
#' @noRd
build_summary_footnote <- function(params, n_sigfig, ofv_decimals = NULL) {
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
        format_hyperion_decimal_string(model_sum$ofv, ofv_decimals)
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
#' Attaches estimation method, OFV, condition number, and number of
#' observations to parameter data for display as the first footnote in
#' the parameter table.
#'
#' @param params Enriched parameter data frame from `apply_table_spec()`
#' @param sum Summary object from `get_model_summary()`, or NULL to skip
#' @param show_method logical, if TRUE adds estimation method attribute for table footnote
#' @param show_ofv logical, if TRUE adds final objective function value attribute for table footnote
#' @param show_cond_num logical, if TRUE adds final condition number attribute for table footnote
#' @param show_number_obs logical, if TRUE adds number of observations attribute for table footnote
#'
#' @return Data frame with model_summary attribute attached
#' @export
add_summary_info <- function(
  params,
  sum,
  show_method = TRUE,
  show_ofv = TRUE,
  show_cond_num = TRUE,
  show_number_obs = TRUE
) {
  if (is.null(sum)) {
    return(params)
  }

  est_method <- if (show_method) {
    dplyr::last(sum$run_details$estimation_method)
  } else {
    NULL
  }

  ofv <- if (show_ofv) {
    dplyr::last(sum$minimization_results$ofv)
  } else {
    NULL
  }

  cn <- if (show_cond_num) {
    dplyr::last(sum$minimization_results$condition_number)
  } else {
    NULL
  }

  n_obs <- if (show_number_obs) {
    dplyr::last(sum$run_details$number_obs)
  } else {
    NULL
  }

  attr(params, "model_summary") <- list(
    estimation_method = est_method,
    ofv = ofv,
    condition_number = cn,
    number_obs = n_obs
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
expand_ci_drop_columns <- function(drop_columns) {
  if (length(drop_columns) == 0) {
    return(drop_columns)
  }

  ci_aliases <- c("ci", "ci_1", "ci_2", "ci_left", "ci_right")
  if (any(drop_columns %in% ci_aliases)) {
    drop_columns <- unique(c(drop_columns, "ci_low", "ci_high"))
  }

  drop_columns
}

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

  drop_columns <- expand_ci_drop_columns(spec@drop_columns)
  add_cols <- spec@add_columns %||% character(0)
  select_cols <- setdiff(spec@columns, drop_columns)
  if (length(add_cols) > 0) {
    select_cols <- unique(c(select_cols, add_cols))
  }
  if ("description" %in% select_cols) {
    select_cols <- c(
      "name",
      "description",
      setdiff(select_cols, c("name", "description"))
    )
  }
  if (
    any(select_cols %in% c("ci_low", "ci_high")) &&
      !"fixed" %in% select_cols
  ) {
    select_cols <- unique(c(select_cols, "fixed"))
  }

  params |>
    dplyr::mutate(
      .appear_order = dplyr::row_number(),
      section = factor(.data$section, levels = section_levels)
    ) |>
    dplyr::arrange(.data$section, .data$.appear_order) |>
    dplyr::select(dplyr::all_of(c(
      select_cols,
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
    stop("TableSpec not found. Run apply_table_spec(params, spec, info) first.")
  }
  params <- order_sections(params, spec)

  # Find columns that are all NA/empty (auto-hide these if enabled)
  empty_cols <- if (spec@hide_empty_columns) {
    find_empty_columns(params)
  } else {
    character(0)
  }

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
  if (!"fixed" %in% spec@columns && !"fixed" %in% spec@add_cols) {
    hide_cols <- c(hide_cols, "fixed")
  }
  hide_cols <- intersect(hide_cols, names(params))

  # Build labels only for columns that exist
  ci_pct <- round(spec@ci_level * 100)
  label_map <- build_parameter_label_map(ci_pct)
  if ("ci_low" %in% spec@columns && !"ci_high" %in% spec@columns) {
    label_map$ci_low <- sprintf("Lower %d%% CI", ci_pct)
  }
  if ("ci_high" %in% spec@columns && !"ci_low" %in% spec@columns) {
    label_map$ci_high <- sprintf("Upper %d%% CI", ci_pct)
  }
  label_map <- label_map[intersect(names(label_map), names(params))]

  # Only use section grouping if sections were defined
  groupname <- if (length(spec@sections) > 0) "section" else NULL

  table <- params |>
    gt::gt(groupname_col = groupname) |>
    gt::cols_hide(dplyr::all_of(hide_cols))

  # CI merge - only if both bounds requested
  if (all(c("ci_low", "ci_high") %in% spec@columns)) {
    table <- merge_ci_columns(table)
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
    apply_gt_missing_text()

  # Format fixed column as "Fixed" or blank
  if ("fixed" %in% c(spec@columns, spec@add_columns)) {
    table <- table |>
      gt::text_transform(
        fn = function(x) ifelse(x == "TRUE", "Fixed", ""),
        locations = gt::cells_body(columns = "fixed")
      )
  }

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
  ofv_decimals <- if (!is.na(spec@n_decimals_ofv)) spec@n_decimals_ofv else NULL
  summary_note <- build_summary_footnote(params, spec@n_sigfig, ofv_decimals)
  if (!is.null(summary_note)) {
    table <- table |> gt::tab_footnote(summary_note)
  }

  # Add conditional footnotes based on what's actually in the table
  table <- add_conditional_footnotes(table, params, spec)

  table <- apply_gt_bold_headers(
    table,
    include_title = TRUE,
    include_row_groups = TRUE
  )

  table <- table |>
    gt::opt_css(css = "td, th { white-space: nowrap; }")

  table
}
