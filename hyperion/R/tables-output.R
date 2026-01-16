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
  fixed_requested <- "fixed" %in% c(spec@columns, add_cols)
  if (fixed_requested && "fixed_fmt" %in% names(params)) {
    select_cols <- unique(c(select_cols, "fixed_fmt"))
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
  # Prepare data + layout (ordering, display columns, labels, hide rules).
  layout <- prepare_parameter_table_data(params, spec)

  # Build gt table and apply base visibility rules.
  table <- layout$params |>
    gt::gt(groupname_col = layout$groupname) |>
    gt::cols_hide(dplyr::all_of(layout$hide_cols))

  # Merge CI bounds when both are requested in the spec.
  if (all(c("ci_low", "ci_high") %in% spec@columns)) {
    table <- merge_ci_columns(table)
  }

  n_sigfig <- spec@n_sigfig
  # Apply labels and numeric formatting.
  table <- apply_standard_gt_formatting(
    table,
    layout$label_map,
    n_sigfig,
    c("estimate", "ci_low", "ci_high", "rse", "shrinkage")
  )

  # Fill CI missing text only for non-fixed rows.
  if (length(layout$ci_rows) > 0) {
    table <- apply_ci_missing_text(
      table,
      c("ci_low", "ci_high"),
      fixed_rows = layout$ci_rows
    )
  }

  # Title + summary + conditional footnotes.
  table <- apply_table_title(table, spec@title)

  # Add summary info as first footnote
  ofv_decimals <- if (!is.na(spec@n_decimals_ofv)) spec@n_decimals_ofv else NULL
  summary_note <- build_summary_footnote(
    layout$params,
    spec@n_sigfig,
    ofv_decimals
  )
  if (!is.null(summary_note)) {
    table <- table |> gt::tab_footnote(summary_note)
  }

  # Add conditional footnotes based on what's actually in the table.
  table <- add_conditional_footnotes(table, layout$params, spec)

  # Final styling (bold headers + nowrap).
  table <- apply_standard_gt_styling(
    table,
    include_title = TRUE,
    include_row_groups = TRUE
  )

  table
}
