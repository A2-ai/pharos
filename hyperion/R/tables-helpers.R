# ==============================================================================
# Table pipeline helpers
# ==============================================================================

#' Prepare parameter table data and layout
#' @noRd
prepare_parameter_table_data <- function(params, spec) {
  params <- blank_ci_for_fixed(params)
  params <- add_fixed_display_columns(params, "fixed")
  params <- order_sections(params, spec)

  empty_cols <- if (spec@hide_empty_columns) {
    find_empty_columns(params)
  } else {
    character(0)
  }

  add_cols <- spec@add_columns %||% character(0)
  requested_cols <- if (isTRUE(spec@.columns_provided)) {
    unique(c(spec@columns, add_cols))
  } else {
    unique(add_cols)
  }
  if ("fixed" %in% requested_cols && "fixed_fmt" %in% names(params)) {
    requested_cols <- unique(c(setdiff(requested_cols, "fixed"), "fixed_fmt"))
  }
  if (spec@hide_empty_columns) {
    empty_cols <- setdiff(empty_cols, requested_cols)
  }

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
    "fixed_fmt",
    dt_cols,
    empty_cols
  )

  fixed_requested <- if (isTRUE(spec@.columns_provided)) {
    "fixed" %in% c(spec@columns, add_cols)
  } else {
    "fixed" %in% add_cols
  }
  if (!fixed_requested) {
    hide_cols <- c(hide_cols, "fixed")
  } else {
    hide_cols <- c(hide_cols, "fixed")
    hide_cols <- setdiff(hide_cols, "fixed_fmt")
  }

  if (
    "fixed" %in%
      names(params) &&
      spec@hide_empty_columns &&
      !any(params$fixed, na.rm = TRUE)
  ) {
    hide_cols <- c(hide_cols, "fixed", "fixed_fmt")
  }
  hide_cols <- intersect(hide_cols, names(params))

  ci_pct <- get_ci_pct(spec, default = 95)
  label_map <- build_parameter_label_map(ci_pct)
  label_map <- adjust_ci_labels(label_map, spec, ci_pct)
  if ("fixed_fmt" %in% names(params)) {
    label_map$fixed_fmt <- label_map$fixed
  }
  label_map <- label_map[intersect(names(label_map), names(params))]

  groupname <- if (length(spec@sections) > 0) "section" else NULL

  ci_rows <- integer(0)
  if (all(c("ci_low", "ci_high") %in% names(params))) {
    ci_rows <- which(!is_fixed_true(params$fixed))
  }

  if (!fixed_requested) {
    hide_cols <- unique(c(hide_cols, "fixed", "fixed_fmt"))
    label_map <- label_map[setdiff(names(label_map), c("fixed", "fixed_fmt"))]
  } else {
    # Always display fixed_fmt, never the raw fixed column.
    hide_cols <- unique(c(hide_cols, "fixed"))
    hide_cols <- setdiff(hide_cols, "fixed_fmt")
    if (!"fixed_fmt" %in% names(label_map) && "fixed" %in% names(label_map)) {
      label_map$fixed_fmt <- label_map$fixed
      label_map$fixed <- NULL
    }
  }

  hide_cols <- intersect(hide_cols, names(params))

  list(
    params = params,
    hide_cols = hide_cols,
    label_map = label_map,
    groupname = groupname,
    ci_rows = ci_rows
  )
}

#' Compute comparison table layout details
#' @noRd
compute_comparison_layout <- function(
  comparison,
  spec,
  suffix_cols,
  model_indices,
  fallback_suffix_cols
) {
  display_cols <- get_comparison_suffix_cols(
    spec,
    comparison,
    fallback_suffix_cols
  )
  display_cols <- setdiff(display_cols, "pct_change")
  fixed_display_cols <- "fixed" %in%
    display_cols &&
    any(grepl("^fixed_fmt_\\d+$", names(comparison)))
  if (fixed_display_cols) {
    display_cols <- sub("^fixed$", "fixed_fmt", display_cols)
  }

  hide_cols <- c("kind", "random_effect", "diagonal", ".appear_order")
  hide_suffix <- grep(
    "^(fixed|fixed_fmt|stderr|variability|shrinkage)_\\d+$",
    names(comparison),
    value = TRUE
  )
  if ("fixed" %in% display_cols || "fixed_fmt" %in% display_cols) {
    hide_suffix <- hide_suffix[!grepl("^fixed_fmt_\\d+$", hide_suffix)]
  }
  if ("stderr" %in% display_cols) {
    hide_suffix <- hide_suffix[!grepl("^stderr_\\d+$", hide_suffix)]
  }
  if ("variability" %in% display_cols) {
    hide_suffix <- hide_suffix[!grepl("^variability_\\d+$", hide_suffix)]
  }
  if ("shrinkage" %in% display_cols) {
    hide_suffix <- hide_suffix[!grepl("^shrinkage_\\d+$", hide_suffix)]
  }
  hide_cols <- intersect(c(hide_cols, hide_suffix), names(comparison))

  add_cols <- if (!is.null(spec)) spec@add_columns %||% character(0) else
    character(0)
  if (!"fixed" %in% c(spec@columns, add_cols)) {
    hide_cols <- unique(c(
      hide_cols,
      grep("^fixed(_\\d+)?$", names(comparison), value = TRUE),
      grep("^fixed_fmt(_\\d+)?$", names(comparison), value = TRUE)
    ))
  } else if (fixed_display_cols) {
    hide_cols <- unique(c(
      hide_cols,
      grep("^fixed_\\d+$", names(comparison), value = TRUE)
    ))
  }

  if (!is.null(spec) && spec@hide_empty_columns) {
    fixed_requested <- if (isTRUE(spec@.columns_provided)) {
      "fixed" %in% c(spec@columns, add_cols)
    } else {
      "fixed" %in% add_cols
    }
    if (!fixed_requested) {
      fixed_cols <- grep("^fixed_\\d+$", names(comparison), value = TRUE)
      for (fc in fixed_cols) {
        if (!any(comparison[[fc]], na.rm = TRUE)) {
          hide_cols <- unique(c(
            hide_cols,
            fc,
            sub("^fixed_", "fixed_fmt_", fc)
          ))
        }
      }
    }

    empty_cols <- find_empty_columns(comparison)
    if (length(display_cols) > 0 && isTRUE(spec@.columns_provided)) {
      requested_suffixes <- unlist(
        lapply(
          display_cols,
          function(col) {
            grep(paste0("^", col, "_\\d+$"), names(comparison), value = TRUE)
          }
        ),
        use.names = FALSE
      )
      empty_cols <- setdiff(empty_cols, requested_suffixes)
    }
    hide_cols <- unique(c(hide_cols, empty_cols))
  }

  pct_change_cols <- grep("^pct_change_\\d+$", names(comparison), value = TRUE)
  columns_provided <- !is.null(spec) && isTRUE(spec@.columns_provided)
  show_pct_change <- !is.null(spec) &&
    ((!columns_provided) ||
      "pct_change" %in% spec@columns ||
      "pct_change" %in% add_cols)
  if (length(pct_change_cols) > 0 && "pct_change" %in% names(comparison)) {
    hide_cols <- unique(c(hide_cols, "pct_change"))
  }
  if (!show_pct_change) {
    hide_cols <- unique(c(hide_cols, pct_change_cols))
    if ("pct_change" %in% names(comparison)) {
      hide_cols <- unique(c(hide_cols, "pct_change"))
    }
  }

  allowed_cols <- display_cols
  if ("ci_low" %in% allowed_cols && !"ci_high" %in% allowed_cols) {
    allowed_cols <- c(allowed_cols, "ci_high")
  }
  allowed_suffixed <- c(
    unlist(
      lapply(allowed_cols, function(col) paste0(col, "_", model_indices)),
      use.names = FALSE
    ),
    if (show_pct_change) pct_change_cols else character(0)
  )
  suffixed_cols <- grep("_(\\d+)$", names(comparison), value = TRUE)
  hide_cols <- unique(c(hide_cols, setdiff(suffixed_cols, allowed_suffixed)))
  if (all(c("ci_low", "ci_high") %in% spec@columns)) {
    hide_cols <- unique(c(
      hide_cols,
      grep("^ci_high_\\d+$", names(comparison), value = TRUE)
    ))
  }

  if (!is.null(spec) && length(spec@drop_columns) > 0) {
    drop_cols <- sub("_left$", "_1", spec@drop_columns)
    drop_cols <- sub("_right$", "_2", drop_cols)
    if (fixed_display_cols && "fixed" %in% drop_cols) {
      drop_cols <- unique(c(drop_cols, "fixed_fmt"))
    }
    drop_suffix <- intersect(drop_cols, suffix_cols)

    drop_expanded <- unlist(
      lapply(
        drop_suffix,
        function(col) paste0(col, "_", model_indices)
      ),
      use.names = FALSE
    )

    if (
      "ci" %in% drop_cols || "ci_low" %in% drop_cols || "ci_high" %in% drop_cols
    ) {
      drop_expanded <- c(
        drop_expanded,
        paste0("ci_low_", model_indices),
        paste0("ci_high_", model_indices)
      )
    }
    if (any(drop_cols %in% c("ci_left", "ci_1"))) {
      drop_expanded <- c(drop_expanded, "ci_low_1", "ci_high_1")
    }
    if (any(drop_cols %in% c("ci_right", "ci_2"))) {
      drop_expanded <- c(drop_expanded, "ci_low_2", "ci_high_2")
    }
    drop_num <- grep("^ci_\\d+$", drop_cols, value = TRUE)
    if (length(drop_num) > 0) {
      nums <- as.integer(sub("^ci_", "", drop_num))
      drop_expanded <- c(
        drop_expanded,
        paste0("ci_low_", nums),
        paste0("ci_high_", nums)
      )
    }
    if ("pct_change" %in% drop_cols) {
      drop_expanded <- c(
        drop_expanded,
        grep("^pct_change(_\\d+)?$", names(comparison), value = TRUE)
      )
    }
    pct_num <- grep("^pct_change_\\d+$", drop_cols, value = TRUE)
    if (length(pct_num) > 0) {
      drop_expanded <- c(drop_expanded, pct_num)
    }
    drop_expanded <- c(drop_expanded, intersect(drop_cols, names(comparison)))
    hide_cols <- unique(c(hide_cols, drop_expanded))
  }

  groupname <- if (
    "section" %in% names(comparison) && !all(is.na(comparison$section))
  ) {
    "section"
  } else {
    NULL
  }

  list(
    display_cols = display_cols,
    hide_cols = hide_cols,
    show_pct_change = show_pct_change,
    pct_change_cols = pct_change_cols,
    fixed_display_cols = fixed_display_cols,
    groupname = groupname
  )
}

#' Compute comparison model columns and reorder data
#' @noRd
compute_comparison_model_cols <- function(
  comparison,
  display_cols,
  model_indices,
  hide_cols,
  spec,
  show_pct_change
) {
  if (all(c("ci_low", "ci_high") %in% display_cols)) {
    display_cols <- display_cols[display_cols != "ci_high"]
  }

  model_cols <- list()
  for (idx in model_indices) {
    cols <- paste0(display_cols, "_", idx)
    cols <- intersect(cols, names(comparison))
    cols <- cols[!cols %in% hide_cols]
    if (all(c("ci_low", "ci_high") %in% spec@columns)) {
      cols <- cols[cols != paste0("ci_high_", idx)]
    }

    pct_col <- paste0("pct_change_", idx)
    if (
      show_pct_change &&
        pct_col %in% names(comparison) &&
        !pct_col %in% hide_cols
    ) {
      cols <- c(cols, pct_col)
    }

    model_cols[[as.character(idx)]] <- cols
  }

  desired_cols <- c("name", unlist(model_cols, use.names = FALSE))
  remaining_cols <- setdiff(names(comparison), desired_cols)
  comparison <- comparison[, c(desired_cols, remaining_cols), drop = FALSE]

  list(comparison = comparison, model_cols = model_cols)
}

#' Build comparison label map
#' @noRd
build_comparison_label_map <- function(
  labels,
  pct_change_cols,
  show_pct_change,
  ci_pct,
  spec,
  fixed_display_cols,
  model_indices,
  comparison,
  hide_cols
) {
  label_map <- list(name = "Parameter", pct_change = "% Change")
  if (length(pct_change_cols) > 0 && show_pct_change) {
    label_map$pct_change <- NULL
    for (col in pct_change_cols) {
      idx <- as.integer(sub("^pct_change_", "", col))
      left_label <- if (length(labels) >= idx - 1) labels[idx - 1] else {
        paste0("Model ", idx - 1)
      }
      if (length(pct_change_cols) == 1) {
        label_map[[col]] <- "% Change"
      } else {
        label_map[[col]] <- sprintf("%% Change vs %s", left_label)
      }
    }
  }

  base_labels <- build_parameter_label_map(ci_pct)
  base_labels <- base_labels[names(base_labels) != "name"]
  base_labels <- adjust_ci_labels(base_labels, spec, ci_pct)
  if (fixed_display_cols) {
    base_labels$fixed_fmt <- base_labels$fixed
    base_labels$fixed <- NULL
  }
  for (idx in model_indices) {
    for (col in names(base_labels)) {
      label_map[[paste0(col, "_", idx)]] <- base_labels[[col]]
    }
  }

  label_map[setdiff(
    intersect(names(label_map), names(comparison)),
    hide_cols
  )]
}

#' Apply model spanners to comparison table
#' @noRd
apply_model_spanners <- function(table, model_cols, labels) {
  for (i in seq_along(model_cols)) {
    cols <- model_cols[[i]]
    if (length(cols) > 0) {
      label <- if (length(labels) >= i) labels[i] else paste0("Model ", i)
      table <- table |>
        gt::tab_spanner(label = label, columns = dplyr::all_of(cols))
    }
  }
  table
}

#' Apply comparison table footnotes
#' @noRd
apply_comparison_footnotes <- function(
  table,
  comparison,
  spec,
  n_sigfig,
  ci_pct
) {
  ofv_decimals <- if (!is.null(spec) && !is.na(spec@n_decimals_ofv)) {
    spec@n_decimals_ofv
  } else {
    NULL
  }
  pvalue_scientific <- if (!is.null(spec)) spec@pvalue_scientific else TRUE
  footnote_lines <- build_comparison_footnote(
    comparison,
    n_sigfig,
    ofv_decimals,
    pvalue_scientific
  )
  if (!is.null(footnote_lines)) {
    for (fn_line in footnote_lines) {
      table <- table |>
        gt::tab_footnote(fn_line)
    }
  }

  ci_cols <- grep("^ci_low_\\d+$", names(comparison), value = TRUE)
  if (length(ci_cols) > 0 && any(!is.na(comparison[ci_cols]))) {
    table <- table |>
      gt::tab_footnote(
        footnote = gt::md(sprintf(
          "%d%% CI: $\\mathrm{Estimate} \\pm z_{%.3g} \\cdot \\mathrm{SE}$",
          ci_pct,
          (1 - ci_pct / 100) / 2
        ))
      )
  }

  comparison_stats <- detect_comparison_statistics(comparison)
  add_conditional_footnotes(table, comparison, spec, comparison_stats)
}

#' Prepare comparison table data and layout
#' @noRd
prepare_comparison_table_data <- function(
  comparison,
  spec,
  fallback_suffix_cols
) {
  suffix_cols <- get_comparison_suffix_cols(
    spec,
    comparison,
    fallback_suffix_cols
  )
  meta <- normalize_comparison_meta(comparison, suffix_cols)
  labels <- meta$labels
  summaries <- meta$summaries
  model_indices <- get_comparison_model_indices(names(comparison), suffix_cols)

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

  attr(comparison, "summary1") <- summaries[[max(1, length(summaries) - 1)]]
  attr(comparison, "summary2") <- summaries[[length(summaries)]]
  attr(comparison, "summaries") <- summaries
  attr(comparison, "labels") <- labels

  comparison <- blank_ci_for_fixed(comparison)
  fixed_cols <- grep("^fixed_\\d+$", names(comparison), value = TRUE)
  comparison <- add_fixed_display_columns(comparison, fixed_cols)

  layout <- compute_comparison_layout(
    comparison,
    spec,
    suffix_cols,
    model_indices,
    fallback_suffix_cols
  )

  list(
    comparison = comparison,
    layout = layout,
    labels = labels,
    summaries = summaries,
    suffix_cols = suffix_cols,
    model_indices = model_indices
  )
}
