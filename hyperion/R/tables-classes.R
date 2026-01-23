# ==============================================================================
# HyperionTable S7 Class - Intermediate Table Representation
# ==============================================================================

#' Border specification for table styling
#'
#' @param sides Character vector of sides ("top", "bottom", "left", "right")
#' @param color Border color (default "#D3D3D3")
#' @param columns Character vector of column names to apply border to
#' @param part Table part ("body", "header", "all")
#' @return List with border specification
#' @noRd
border_spec <- function(
  sides = "right",
  color = "#D3D3D3",
  columns = character(0),
  part = "body"
) {
  list(
    sides = sides,
    color = color,
    columns = columns,
    part = part
  )
}

#' Spanner specification for column grouping
#'
#' @param label Spanner label text
#' @param columns Character vector of column names under this spanner
#' @return List with spanner specification
#' @noRd
spanner_spec <- function(label, columns) {
  list(
    label = label,
    columns = columns
  )
}

#' Footnote specification
#'
#' @param content Footnote text (can be plain text or markdown)
#' @param is_markdown Whether content should be rendered as markdown
#' @return List with footnote specification
#' @noRd
footnote_spec <- function(content, is_markdown = FALSE) {
  list(
    content = content,
    is_markdown = is_markdown
  )
}

#' CI merge specification
#'
#' @param ci_low Name of the lower CI column
#' @param ci_high Name of the upper CI column
#' @param pattern Merge pattern (default `"[{1}, {2}]"`)
#' @return List with CI merge specification
#' @noRd
ci_merge_spec <- function(
  ci_low = "ci_low",
  ci_high = "ci_high",
  pattern = "[{1}, {2}]"
) {
  list(
    ci_low = ci_low,
    ci_high = ci_high,
    pattern = pattern
  )
}

#' HyperionTable - Intermediate representation for table rendering
#'
#' A declarative table specification that can be rendered to multiple output
#' formats (gt, flextable). Captures all styling intent in a format-agnostic way.
#'
#' @slot data Data frame containing the table data
#' @slot table_type Character string: "parameter", "comparison", or "summary"
#' @slot groupname_col Column name for row grouping (NULL for no grouping)
#' @slot hide_cols Character vector of columns to hide
#' @slot col_labels Named list mapping column names to display labels
#' @slot title Table title (NULL for no title)
#' @slot spanners List of spanner specifications for column grouping
#' @slot numeric_cols Character vector of columns to format as numeric
#' @slot n_sigfig Number of significant figures for numeric formatting
#' @slot ci_merges List of CI merge specifications
#' @slot ci_missing_text Text to show for missing CI values (default "-")
#' @slot missing_text Text to show for other missing values (default "")
#' @slot bold_locations Character vector of locations to bold
#' @slot borders List of border specifications
#' @slot footnotes List of footnote specifications (in order)
#' @slot source_spec Original TableSpec/SummarySpec (for reference)
#'
#' @export
HyperionTable <- S7::new_class(
  "HyperionTable",
  properties = list(
    # Core data
    data = S7::class_data.frame,

    # Metadata
    table_type = S7::new_property(
      class = S7::class_character,
      default = "parameter"
    ),

    # Structure
    groupname_col = S7::new_property(
      class = S7::class_character | NULL,
      default = NULL
    ),
    hide_cols = S7::new_property(
      class = S7::class_character,
      default = character(0)
    ),

    # Labels & Headers
    col_labels = S7::new_property(
      class = S7::class_list,
      default = list()
    ),
    title = S7::new_property(
      class = S7::class_character | NULL,
      default = NULL
    ),
    spanners = S7::new_property(
      class = S7::class_list,
      default = list()
    ),

    # Formatting
    numeric_cols = S7::new_property(
      class = S7::class_character,
      default = character(0)
    ),
    n_sigfig = S7::new_property(
      class = S7::class_numeric,
      default = 3
    ),
    ci_merges = S7::new_property(
      class = S7::class_list,
      default = list()
    ),
    ci_missing_rows = S7::new_property(
      # Row indices where CI missing text should show "-"
      class = S7::class_integer,
      default = integer(0)
    ),
    ci_missing_text = S7::new_property(
      class = S7::class_character,
      default = "-"
    ),
    missing_text = S7::new_property(
      class = S7::class_character,
      default = ""
    ),

    # Styling
    bold_locations = S7::new_property(
      # "column_labels", "title", "row_groups", "spanners"
      class = S7::class_character,
      default = c("column_labels")
    ),
    borders = S7::new_property(
      class = S7::class_list,
      default = list()
    ),

    # Footnotes (in order)
    footnotes = S7::new_property(
      class = S7::class_list,
      default = list()
    ),

    # Reference to original spec
    source_spec = S7::new_property(
      class = S7::class_any,
      default = NULL
    )
  ),
  validator = function(self) {
    valid_types <- c("parameter", "comparison", "summary")
    if (!self@table_type %in% valid_types) {
      return(sprintf(
        "@table_type must be one of: %s. Got: %s",
        paste(valid_types, collapse = ", "),
        self@table_type
      ))
    }

    valid_bold <- c("column_labels", "title", "row_groups", "spanners")
    bad_bold <- setdiff(self@bold_locations, valid_bold)
    if (length(bad_bold) > 0) {
      return(sprintf(
        "@bold_locations must be in: %s. Got: %s",
        paste(valid_bold, collapse = ", "),
        paste(bad_bold, collapse = ", ")
      ))
    }
  }
)

# ==============================================================================
# HyperionTable Constructors
# ==============================================================================

#' Create HyperionTable from parameter table layout
#'
#' @param params Data frame from prepare_parameter_table_data()
#' @param layout List with hide_cols, label_map, groupname, ci_rows
#' @param spec TableSpec object
#' @return HyperionTable object
#' @noRd
hyperion_parameter_table <- function(params, layout, spec) {
  # Determine CI merge columns
  ci_in_spec <- all(c("ci_low", "ci_high") %in% spec@columns)
  ci_in_data <- all(c("ci_low", "ci_high") %in% names(params))
  ci_merges <- if (ci_in_spec && ci_in_data) {
    list(ci_merge_spec("ci_low", "ci_high"))
  } else {
    list()
  }

  # Build footnotes
  footnotes <- build_parameter_footnotes(params, spec, layout)

  # Numeric columns for formatting
  numeric_cols <- intersect(
    c("estimate", "ci_low", "ci_high", "rse", "shrinkage"),
    names(params)
  )

  # Bold locations
  bold_locs <- c("column_labels")
  if (!is.null(spec@title) && nchar(spec@title) > 0) {
    bold_locs <- c(bold_locs, "title")
  }
  if (!is.null(layout$groupname)) {
    bold_locs <- c(bold_locs, "row_groups")
  }

  HyperionTable(
    data = params,
    table_type = "parameter",
    groupname_col = layout$groupname,
    hide_cols = layout$hide_cols,
    col_labels = layout$label_map,
    title = spec@title,
    spanners = list(),
    numeric_cols = numeric_cols,
    n_sigfig = spec@n_sigfig,
    ci_merges = ci_merges,
    ci_missing_rows = as.integer(layout$ci_rows),
    bold_locations = bold_locs,
    borders = list(),
    footnotes = footnotes,
    source_spec = spec
  )
}

#' Create HyperionTable from comparison table layout
#'
#' @param comparison Data frame from prepare_comparison_table_data()
#' @param layout List with display_cols, hide_cols, etc.
#' @param model_cols List of columns per model
#' @param labels Character vector of model labels
#' @param spec TableSpec object
#' @param label_map Named list of column labels
#' @param model_indices Integer vector of model indices
#' @return HyperionTable object
#' @noRd
hyperion_comparison_table <- function(
  comparison,
  layout,
  model_cols,
  labels,
  spec,
  label_map,
  model_indices
) {
  # Build spanners from model_cols
  spanners <- list()
  for (i in seq_along(model_cols)) {
    cols <- model_cols[[i]]
    if (length(cols) > 0) {
      label <- if (length(labels) >= i) labels[i] else paste0("Model ", i)
      spanners <- c(spanners, list(spanner_spec(label, cols)))
    }
  }

  # CI merges per model
  ci_merges <- list()
  if (!is.null(spec) && all(c("ci_low", "ci_high") %in% spec@columns)) {
    for (idx in model_indices) {
      ci_low <- paste0("ci_low_", idx)
      ci_high <- paste0("ci_high_", idx)
      if (all(c(ci_low, ci_high) %in% names(comparison))) {
        ci_merges <- c(ci_merges, list(ci_merge_spec(ci_low, ci_high)))
      }
    }
  }

  # Build border specs
  borders <- list()
  border_cols <- character(0)
  for (cols in model_cols) {
    if (length(cols) > 0) {
      border_cols <- c(border_cols, utils::tail(cols, 1))
    }
  }
  if (length(border_cols) > 0) {
    borders <- list(border_spec(
      sides = "right",
      color = "#D3D3D3",
      columns = border_cols,
      part = "body"
    ))
  }

  # Build footnotes
  footnotes <- build_comparison_footnotes(comparison, spec, layout)

  # Numeric columns
  pct_change_cols <- grep("^pct_change", names(comparison), value = TRUE)
  numeric_cols <- c(
    paste0("estimate_", model_indices),
    paste0("rse_", model_indices),
    paste0("ci_low_", model_indices),
    paste0("ci_high_", model_indices),
    pct_change_cols
  )
  numeric_cols <- intersect(numeric_cols, names(comparison))

  # Bold locations
  bold_locs <- c("column_labels", "spanners")
  if (!is.null(layout$groupname)) {
    bold_locs <- c(bold_locs, "row_groups")
  }

  HyperionTable(
    data = comparison,
    table_type = "comparison",
    groupname_col = layout$groupname,
    hide_cols = layout$hide_cols,
    col_labels = label_map,
    title = spec@title,
    spanners = spanners,
    numeric_cols = numeric_cols,
    n_sigfig = spec@n_sigfig,
    ci_merges = ci_merges,
    ci_missing_rows = integer(0),
    bold_locations = bold_locs,
    borders = borders,
    footnotes = footnotes,
    source_spec = spec
  )
}

#' Create HyperionTable from summary table data
#'
#' @param data Data frame from apply_summary_spec()
#' @param spec SummarySpec object
#' @return HyperionTable object
#' @noRd
hyperion_summary_table <- function(data, spec) {
  # Build label map
  label_map <- build_summary_label_map()
  label_map <- label_map[intersect(names(label_map), names(data))]

  # Add time format suffix
  time_suffix <- get_time_suffix(spec@time_format, data)
  if (!is.null(time_suffix)) {
    time_cols <- intersect(
      c("estimation_time", "covariance_time", "postprocess_time"),
      names(data)
    )
    for (col in time_cols) {
      if (col %in% names(label_map)) {
        label_map[[col]] <- paste0(label_map[[col]], " (", time_suffix, ")")
      }
    }
  }

  # Find empty columns to hide
  empty_cols <- if (spec@hide_empty_columns) find_empty_columns(data) else
    character(0)

  # Build footnotes
  footnotes <- build_summary_footnotes(data, spec)

  # Numeric columns
  ofv_cols <- intersect(c("ofv", "dofv"), names(data))
  sigfig_cols <- intersect(
    c(
      "condition_number",
      "estimation_time",
      "covariance_time",
      "postprocess_time"
    ),
    names(data)
  )

  HyperionTable(
    data = data,
    table_type = "summary",
    groupname_col = NULL,
    hide_cols = empty_cols,
    col_labels = label_map,
    title = spec@title,
    spanners = list(),
    numeric_cols = c(ofv_cols, sigfig_cols),
    n_sigfig = spec@n_sigfig,
    ci_merges = list(),
    ci_missing_rows = integer(0),
    bold_locations = c("column_labels", "title"),
    borders = list(),
    footnotes = footnotes,
    source_spec = spec
  )
}

# ==============================================================================
# Footnote Building Helpers
# ==============================================================================

#' Build footnotes for parameter table
#' @noRd
build_parameter_footnotes <- function(params, spec, layout) {
  footnotes <- list()

  # Get footnote order from spec
  footnote_order <- spec@footnote_order
  if (is.null(footnote_order)) {
    return(list())
  }

  # Build summary info footnote
  ofv_decimals <- if (!is.na(spec@n_decimals_ofv)) spec@n_decimals_ofv else NULL
  summary_note <- build_summary_footnote(params, spec@n_sigfig, ofv_decimals)

  # Detect statistics
  stats <- detect_table_statistics(params)
  ci_pct <- round(spec@ci_level * 100)

  # Check if CI dropped
  expanded_drop <- expand_ci_drop_columns(spec@drop_columns)
  if (all(c("ci_low", "ci_high") %in% expanded_drop)) {
    stats$has_ci <- FALSE
  }

  # Build equations
  equations <- build_equations_footnote(stats, ci_pct, NULL, NULL)

  # Build abbreviations
  abbreviations <- build_abbreviations_footnote(stats, NULL, NULL)

  # Assemble footnotes in order
  for (section in footnote_order) {
    if (section == "summary_info" && !is.null(summary_note)) {
      footnotes <- c(footnotes, list(footnote_spec(summary_note, FALSE)))
    } else if (section == "equations" && !is.null(equations)) {
      for (eq in equations) {
        # equations are gt::md() objects, extract the content
        content <- if (inherits(eq, "gt_md")) as.character(eq) else eq
        footnotes <- c(footnotes, list(footnote_spec(content, TRUE)))
      }
    } else if (section == "abbreviations" && !is.null(abbreviations)) {
      for (line in abbreviations) {
        footnotes <- c(footnotes, list(footnote_spec(line, FALSE)))
      }
    }
  }

  footnotes
}

#' Build footnotes for comparison table
#' @noRd
build_comparison_footnotes <- function(comparison, spec, layout) {
  footnotes <- list()

  footnote_order <- spec@footnote_order
  if (is.null(footnote_order)) {
    return(list())
  }

  n_sigfig <- spec@n_sigfig
  ci_pct <- round(spec@ci_level * 100)

  # Build summary note
  ofv_decimals <- if (!is.na(spec@n_decimals_ofv)) spec@n_decimals_ofv else NULL
  pvalue_scientific <- spec@pvalue_scientific
  pvalue_threshold <- spec@pvalue_threshold
  summary_note <- build_comparison_footnote(
    comparison,
    n_sigfig,
    ofv_decimals,
    pvalue_scientific,
    pvalue_threshold
  )

  # Detect statistics
  stats <- detect_table_statistics(comparison)
  comparison_stats <- detect_comparison_statistics(comparison)

  # Check if CI dropped
  expanded_drop <- expand_ci_drop_columns(spec@drop_columns)
  if (all(c("ci_low", "ci_high") %in% expanded_drop)) {
    stats$has_ci <- FALSE
  }

  # Build equations
  equations <- build_equations_footnote(stats, ci_pct, comparison_stats, NULL)

  # Build abbreviations
  abbreviations <- build_abbreviations_footnote(stats, comparison_stats, NULL)

  # Assemble footnotes in order
  for (section in footnote_order) {
    if (section == "summary_info" && !is.null(summary_note)) {
      for (line in summary_note) {
        footnotes <- c(footnotes, list(footnote_spec(line, FALSE)))
      }
    } else if (section == "equations" && !is.null(equations)) {
      for (eq in equations) {
        content <- if (inherits(eq, "gt_md")) as.character(eq) else eq
        footnotes <- c(footnotes, list(footnote_spec(content, TRUE)))
      }
    } else if (section == "abbreviations" && !is.null(abbreviations)) {
      for (line in abbreviations) {
        footnotes <- c(footnotes, list(footnote_spec(line, FALSE)))
      }
    }
  }

  footnotes
}

#' Build footnotes for summary table
#' @noRd
build_summary_footnotes <- function(data, spec) {
  footnotes <- list()

  footnote_order <- spec@footnote_order
  if (is.null(footnote_order)) {
    return(list())
  }

  # Summary stats for conditional footnotes
  summary_stats <- list(
    has_ofv = "ofv" %in% names(data) && any(!is.na(data$ofv)),
    has_dofv = "dofv" %in% names(data) && any(!is.na(data$dofv)),
    has_cond_num = "condition_number" %in%
      names(data) &&
      any(!is.na(data$condition_number)),
    has_pvalue = "pvalue" %in% names(data) && any(!is.na(data$pvalue)),
    dofv_excluded = isTRUE(attr(data, "dofv_excluded_nobs_mismatch"))
  )

  # Detect table statistics (empty for summary tables)
  stats <- list(
    has_ci = FALSE,
    has_rse = FALSE,
    has_stderr = FALSE,
    has_cv = FALSE,
    has_sd = FALSE,
    has_corr = FALSE,
    has_shrinkage = FALSE,
    has_theta_logadderr_cv = FALSE,
    has_omega_lognormal_cv = FALSE,
    has_omega_proportional_cv = FALSE,
    has_sigma_lognormal_cv = FALSE,
    has_sigma_proportional_cv = FALSE
  )

  # Build abbreviations
  abbreviations <- build_abbreviations_footnote(stats, NULL, summary_stats)

  # Assemble footnotes
  for (section in footnote_order) {
    if (section == "abbreviations" && !is.null(abbreviations)) {
      for (line in abbreviations) {
        footnotes <- c(footnotes, list(footnote_spec(line, FALSE)))
      }
    }
  }

  footnotes
}
