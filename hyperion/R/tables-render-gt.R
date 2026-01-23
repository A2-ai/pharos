# ==============================================================================
# GT Rendering for HyperionTable
# ==============================================================================

#' Render HyperionTable as gt table
#'
#' Converts a HyperionTable intermediate representation to a gt table object.
#'
#' @param table A HyperionTable object
#' @return A gt table object
#' @export
render_gt <- function(table) {
  if (!S7::S7_inherits(table, HyperionTable)) {
    stop("table must be a HyperionTable object")
  }
  if (!requireNamespace("gt", quietly = TRUE)) {
    stop("Package 'gt' is required for render_gt()")
  }
  if (!requireNamespace("dplyr", quietly = TRUE)) {
    stop("Package 'dplyr' is required for render_gt()")
  }

  payload <- prepare_table_render(table)

  # Create base gt table
  gt_table <- gt::gt(payload$data, groupname_col = table@groupname_col)

  # Hide columns that remain in the prepared data
  gt_table <- apply_gt_hide_cols(gt_table, table, names(payload$data))

  # Apply column labels
  gt_table <- apply_gt_labels(gt_table, table, payload$visible_cols)

  # Apply spanners
  gt_table <- apply_gt_spanners(gt_table, table, payload$visible_cols)

  # Format markdown
  gt_table <- gt_table |>
    gt::fmt_markdown()

  # Add title
  gt_table <- apply_gt_title(gt_table, table)

  # Apply bold styling
  gt_table <- apply_gt_bold(gt_table, table)

  # Apply borders
  gt_table <- apply_gt_borders(gt_table, table, payload$visible_cols)

  # Add footnotes
  gt_table <- apply_gt_footnotes(gt_table, table)

  # Add nowrap CSS
  gt_table <- gt_table |>
    gt::opt_css(css = "td, th { white-space: nowrap; }")

  gt_table
}

#' Hide columns in gt table
#' @noRd
apply_gt_hide_cols <- function(gt_table, table, data_cols) {
  if (length(table@hide_cols) == 0) {
    return(gt_table)
  }

  hide_cols <- intersect(table@hide_cols, data_cols)
  if (length(hide_cols) == 0) {
    return(gt_table)
  }

  gt_table |>
    gt::cols_hide(dplyr::all_of(hide_cols))
}

#' Apply column labels to gt table
#' @noRd
apply_gt_labels <- function(gt_table, table, visible_cols) {
  if (length(table@col_labels) == 0) {
    return(gt_table)
  }

  labels_to_apply <- table@col_labels[
    intersect(names(table@col_labels), visible_cols)
  ]

  if (length(labels_to_apply) == 0) {
    return(gt_table)
  }

  gt_table |>
    gt::cols_label(!!!labels_to_apply)
}

#' Format numeric columns in gt table
#' @noRd
apply_gt_numeric_format <- function(gt_table, table) {
  numeric_cols <- intersect(table@numeric_cols, names(table@data))

  if (length(numeric_cols) == 0) {
    return(gt_table)
  }

  gt_table |>
    gt::fmt_number(
      columns = dplyr::any_of(numeric_cols),
      n_sigfig = table@n_sigfig
    )
}

#' Apply missing text substitution to gt table
#' @noRd
apply_gt_missing <- function(gt_table, table) {
  gt_table |>
    gt::sub_missing(
      columns = dplyr::everything(),
      missing_text = table@missing_text
    )
}

#' Add title to gt table
#' @noRd
apply_gt_title <- function(gt_table, table) {
  if (is.null(table@title) || nchar(table@title) == 0) {
    return(gt_table)
  }

  gt_table |>
    gt::tab_header(title = table@title)
}

#' Apply CI column merges to gt table
#' @noRd
apply_gt_ci_merges <- function(gt_table, table) {
  for (merge in table@ci_merges) {
    ci_low <- merge$ci_low
    ci_high <- merge$ci_high
    if (all(c(ci_low, ci_high) %in% names(table@data))) {
      gt_table <- gt_table |>
        gt::cols_merge(
          columns = c(ci_low, ci_high),
          pattern = merge$pattern,
          rows = !is.na(.data[[ci_low]]) & !is.na(.data[[ci_high]])
        )
    }
  }
  gt_table
}

#' Apply spanners to gt table
#' @noRd
apply_gt_spanners <- function(gt_table, table, visible_cols) {
  for (spanner in table@spanners) {
    cols <- intersect(spanner$columns, visible_cols)
    if (length(cols) > 0) {
      gt_table <- gt_table |>
        gt::tab_spanner(label = spanner$label, columns = dplyr::all_of(cols))
    }
  }
  gt_table
}

#' Apply CI missing text to gt table
#' @noRd
apply_gt_ci_missing <- function(gt_table, table) {
  if (length(table@ci_missing_rows) == 0) {
    return(gt_table)
  }

  # Find CI columns
  ci_cols <- character(0)
  for (merge in table@ci_merges) {
    ci_cols <- c(ci_cols, merge$ci_low, merge$ci_high)
  }
  ci_cols <- intersect(ci_cols, names(table@data))

  if (length(ci_cols) > 0) {
    gt_table <- gt_table |>
      gt::sub_missing(
        columns = dplyr::all_of(ci_cols),
        rows = table@ci_missing_rows,
        missing_text = table@ci_missing_text
      )
  }

  gt_table
}

#' Apply bold styling to gt table
#' @noRd
apply_gt_bold <- function(gt_table, table) {
  locations <- list()

  if ("column_labels" %in% table@bold_locations) {
    locations <- c(
      locations,
      list(gt::cells_column_labels(dplyr::everything()))
    )
  }
  if ("title" %in% table@bold_locations) {
    locations <- c(locations, list(gt::cells_title(groups = "title")))
  }
  if ("row_groups" %in% table@bold_locations) {
    locations <- c(locations, list(gt::cells_row_groups()))
  }
  if ("spanners" %in% table@bold_locations) {
    locations <- c(
      locations,
      list(gt::cells_column_spanners(dplyr::everything()))
    )
  }

  if (length(locations) > 0) {
    gt_table <- gt_table |>
      gt::tab_style(
        style = gt::cell_text(weight = "bold"),
        locations = locations
      )
  }

  gt_table
}

#' Apply borders to gt table
#' @noRd
apply_gt_borders <- function(gt_table, table, visible_cols) {
  for (border in table@borders) {
    cols <- intersect(border$columns, visible_cols)
    if (length(cols) > 0) {
      gt_table <- gt_table |>
        gt::tab_style(
          style = gt::cell_borders(sides = border$sides, color = border$color),
          locations = gt::cells_body(columns = dplyr::all_of(cols))
        )
    }
  }
  gt_table
}

#' Apply footnotes to gt table
#' @noRd
apply_gt_footnotes <- function(gt_table, table) {
  for (fn in table@footnotes) {
    content <- if (fn$is_markdown) gt::md(fn$content) else fn$content
    gt_table <- gt_table |>
      gt::tab_footnote(content)
  }
  gt_table
}

# ==============================================================================
# Summary Table Special Handling
# ==============================================================================

#' Render summary HyperionTable as gt table
#'
#' Specialized rendering for summary tables which have additional formatting
#' requirements (OFV decimals, pvalue/df merge, etc.)
#'
#' @param table A HyperionTable object with table_type = "summary"
#' @return A gt table object
#' @noRd
render_gt_summary <- function(table) {
  if (!S7::S7_inherits(table, HyperionTable)) {
    stop("table must be a HyperionTable object")
  }
  if (table@table_type != "summary") {
    stop("table@table_type must be 'summary'")
  }

  spec <- table@source_spec
  data <- table@data

  # Pre-merge pvalue with df before creating table
  if (all(c("pvalue", "df") %in% names(data))) {
    use_scientific <- spec@pvalue_scientific
    pval_threshold <- spec@pvalue_threshold
    n_sig <- spec@n_sigfig
    data$pvalue <- vapply(
      seq_len(nrow(data)),
      function(i) {
        pval <- data$pvalue[i]
        df_val <- data$df[i]
        if (is.na(pval) || is.na(df_val)) {
          return(NA_character_)
        }
        format_p <- format_pvalue_string(
          pval,
          n_sig,
          use_scientific,
          pval_threshold
        )
        sprintf("%s (df = %d)", format_p, df_val)
      },
      character(1)
    )
    data$df <- NULL
    table@data <- data
  }

  # Use standard rendering
  gt_table <- render_gt(table)

  # Apply OFV-specific formatting
  ofv_cols <- intersect(c("ofv", "dofv"), names(data))
  if (length(ofv_cols) > 0) {
    gt_table <- gt_table |>
      gt::fmt_number(
        columns = dplyr::all_of(ofv_cols),
        decimals = spec@n_decimals_ofv
      )
  }

  gt_table
}
