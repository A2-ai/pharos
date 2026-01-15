# ==============================================================================
# Model comparison functions
# ==============================================================================

#' @noRd
get_comparison_model_indices <- function(names_vec, suffix_cols) {
  pattern <- paste0("^(", paste(suffix_cols, collapse = "|"), ")_(\\d+)$")
  matched <- grep(pattern, names_vec, value = TRUE)
  if (length(matched) == 0) {
    return(integer(0))
  }
  indices <- as.integer(sub(pattern, "\\2", matched))
  indices <- sort(unique(indices[!is.na(indices)]))
  indices
}

#' @noRd
normalize_comparison_meta <- function(comparison, suffix_cols) {
  labels <- attr(comparison, "labels")
  summaries <- attr(comparison, "summaries")

  if (is.null(labels)) {
    indices <- get_comparison_model_indices(names(comparison), suffix_cols)
    labels <- paste0("Model ", indices)
  }

  if (is.null(summaries)) {
    sum1 <- attr(comparison, "summary1")
    sum2 <- attr(comparison, "summary2")
    summaries <- list(sum1, sum2)
  }

  list(labels = labels, summaries = summaries)
}

#' @noRd
get_comparison_suffix_cols <- function(
  spec,
  params,
  fallback_cols,
  include_fixed_for_ci = FALSE
) {
  if (!is.null(spec) && !is.null(spec@columns)) {
    cols <- setdiff(spec@columns, "name")
  } else {
    cols <- fallback_cols
  }

  if (include_fixed_for_ci && any(cols %in% c("ci_low", "ci_high"))) {
    cols <- unique(c(cols, "fixed"))
  }

  cols <- cols[cols != "pct_change"]

  if (inherits(params, "hyperion_comparison")) {
    cols <- cols[vapply(
      cols,
      function(col) any(grepl(paste0("^", col, "_\\d+$"), names(params))),
      logical(1)
    )]
  } else {
    cols <- intersect(cols, names(params))
  }

  cols
}

#' Compare two enriched parameter data frames
#'
#' Joins two enriched parameter data frames for side-by-side comparison.
#' Both inputs should be prepared using the standard pipeline:
#' `get_parameters() |> apply_table_spec() |> add_summary_info()`.
#' Can also be chained by passing an existing `hyperion_comparison` object as
#' `params1` to add another model comparison.
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

  # Columns to suffix (model-specific values)
  fallback_suffix_cols <- c(
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

  # Extract attributes from both dataframes
  spec1 <- attr(params1, "table_spec")
  spec2 <- attr(params2, "table_spec")
  sum2 <- attr(params2, "model_summary")
  suffix_cols <- get_comparison_suffix_cols(
    spec1,
    params1,
    fallback_suffix_cols,
    include_fixed_for_ci = TRUE
  )
  add_cols1 <- if (!is.null(spec1)) spec1@add_columns %||% character(0) else
    character(0)
  columns_provided <- !is.null(spec1) && isTRUE(spec1@columns_provided)
  if (is.null(spec1) || !columns_provided) {
    suffix_cols <- unique(c(suffix_cols, "pct_change"))
  } else if ("pct_change" %in% add_cols1) {
    suffix_cols <- unique(c(suffix_cols, "pct_change"))
  }

  is_comparison <- inherits(params1, "hyperion_comparison")
  if (is_comparison) {
    meta <- normalize_comparison_meta(params1, suffix_cols)
    existing_labels <- meta$labels
    existing_summaries <- meta$summaries
    model_indices <- get_comparison_model_indices(names(params1), suffix_cols)
    max_index <- if (length(model_indices) > 0) max(model_indices) else 0
    model_count <- if (max_index > 0) max_index else
      max(
        length(existing_labels),
        length(existing_summaries)
      )
    if (length(existing_summaries) < model_count) {
      existing_summaries <- c(
        existing_summaries,
        rep(list(NULL), model_count - length(existing_summaries))
      )
    }
    if (length(existing_labels) < model_count) {
      existing_labels <- c(
        existing_labels,
        paste0("Model ", (length(existing_labels) + 1):model_count)
      )
    }
  } else {
    existing_labels <- NULL
    existing_summaries <- NULL
    model_count <- 1
  }

  # Validate labels
  if (is_comparison) {
    if (length(labels) == 1) {
      labels <- c(existing_labels, labels)
    } else if (length(labels) == 2) {
      if (length(existing_labels) > 0) {
        existing_labels[length(existing_labels)] <- labels[1]
      }
      labels <- c(existing_labels, labels[2])
    } else {
      stop(
        "labels must be length 1 or 2 when comparing with an existing comparison"
      )
    }
  } else if (length(labels) != 2) {
    stop("labels must be a character vector of length 2")
  }

  # Extract attributes from both dataframes
  sum1 <- if (is_comparison) utils::tail(existing_summaries, 1)[[1]] else {
    attr(params1, "model_summary")
  }

  # Warn if missing attributes but don't fail
  if (!is_comparison && is.null(sum1)) {
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
  spec <- if (!is.null(spec1)) spec1 else spec2

  next_index <- if (is_comparison) model_count + 1 else 2

  # Select relevant columns from params2
  keep_cols <- c("name", suffix_cols, coalesce_cols)
  keep_cols2 <- intersect(keep_cols, names(params2))
  p2 <- params2[, keep_cols2, drop = FALSE]

  # Rename suffix columns for params2
  for (col in suffix_cols) {
    if (col %in% names(p2)) {
      names(p2)[names(p2) == col] <- paste0(col, "_", next_index)
    }
  }

  if (is_comparison) {
    # Base comparison keeps all existing model-specific columns
    base_suffix_pattern <- paste0(
      "^(",
      paste(suffix_cols, collapse = "|"),
      ")_\\d+$"
    )
    pct_pattern <- "^pct_change(_\\d+)?$"
    keep_base <- unique(c(
      "name",
      grep(base_suffix_pattern, names(params1), value = TRUE),
      grep(pct_pattern, names(params1), value = TRUE)
    ))
    base_suffix <- params1[, keep_base, drop = FALSE]

    base_coalesce <- params1[,
      intersect(c("name", coalesce_cols), names(params1)),
      drop = FALSE
    ]
    p2_coalesce <- p2[,
      intersect(c("name", coalesce_cols), names(p2)),
      drop = FALSE
    ]

    comparison <- dplyr::full_join(base_suffix, p2, by = "name")

    coalesce_df <- dplyr::full_join(
      base_coalesce,
      p2_coalesce,
      by = "name",
      suffix = c("_prev", "_new")
    )
    for (col in coalesce_cols) {
      col_prev <- paste0(col, "_prev")
      col_new <- paste0(col, "_new")
      if (col_prev %in% names(coalesce_df) || col_new %in% names(coalesce_df)) {
        coalesce_df[[col]] <- dplyr::coalesce(
          coalesce_df[[col_prev]],
          coalesce_df[[col_new]]
        )
        coalesce_df[[col_prev]] <- NULL
        coalesce_df[[col_new]] <- NULL
      }
    }
    comparison <- comparison[,
      setdiff(names(comparison), coalesce_cols),
      drop = FALSE
    ]
    comparison <- dplyr::left_join(comparison, coalesce_df, by = "name")
  } else {
    # Select relevant columns from each dataframe
    keep_cols1 <- intersect(keep_cols, names(params1))
    p1 <- params1[, keep_cols1, drop = FALSE]

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
  }

  # Calculate percent change: (estN - estN-1) / estN-1 * 100
  if (is_comparison) {
    last_idx <- next_index
    prev_idx <- if (length(model_indices) > 0) max(model_indices) else
      next_index - 1
  } else {
    last_idx <- 2
    prev_idx <- 1
  }
  est_prev <- paste0("estimate_", prev_idx)
  est_last <- paste0("estimate_", last_idx)
  pct_col <- paste0("pct_change_", last_idx)
  if (est_prev %in% names(comparison) && est_last %in% names(comparison)) {
    comparison[[pct_col]] <- dplyr::case_when(
      is.na(comparison[[est_prev]]) | is.na(comparison[[est_last]]) ~ NA_real_,
      comparison[[est_prev]] == 0 ~ NA_real_,
      TRUE ~
        (comparison[[est_last]] - comparison[[est_prev]]) /
          comparison[[est_prev]] *
          100
    )
    comparison$pct_change <- comparison[[pct_col]]
  }

  # Attach class and attributes
  class(comparison) <- c("hyperion_comparison", class(comparison))
  if (is_comparison) {
    summaries <- c(existing_summaries, list(sum2))
  } else {
    summaries <- list(sum1, sum2)
  }
  last_two <- utils::tail(summaries, 2)
  attr(comparison, "summary1") <- last_two[[1]]
  attr(comparison, "summary2") <- last_two[[2]]
  attr(comparison, "summaries") <- summaries
  attr(comparison, "labels") <- labels
  attr(comparison, "table_spec") <- spec

  comparison
}

#' Detect which statistics are present in a comparison table
#'
#' @param comparison Data frame from compare_with()
#' @return Named list of logicals indicating which stats are present
#' @noRd
get_comparison_last_two <- function(comparison, suffix_cols) {
  meta <- normalize_comparison_meta(comparison, suffix_cols)
  labels <- meta$labels
  summaries <- meta$summaries
  if (length(labels) < 2) {
    labels <- c(labels, "Model")
  }
  if (length(summaries) < 2) {
    summaries <- c(summaries, list(NULL))
  }
  list(
    labels = utils::tail(labels, 2),
    summaries = utils::tail(summaries, 2)
  )
}

detect_comparison_statistics <- function(comparison) {
  fallback_suffix_cols <- c(
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
  spec <- attr(comparison, "table_spec")
  suffix_cols <- get_comparison_suffix_cols(
    spec,
    comparison,
    fallback_suffix_cols
  )
  last_two <- get_comparison_last_two(comparison, suffix_cols)
  sum1 <- last_two$summaries[[1]]
  sum2 <- last_two$summaries[[2]]

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
      model_indices <- get_comparison_model_indices(
        names(comparison),
        suffix_cols
      )
      if (length(model_indices) > 1) {
        last_idx <- utils::tail(model_indices, 1)
        prev_idx <- model_indices[length(model_indices) - 1]
        fixed1 <- comparison[[paste0("fixed_", prev_idx)]]
        fixed2 <- comparison[[paste0("fixed_", last_idx)]]
      } else {
        fixed1 <- NULL
        fixed2 <- NULL
      }
      if (!is.null(fixed1) && !is.null(fixed2)) {
        k1 <- sum(!is.na(fixed1) & !fixed1, na.rm = TRUE)
        k2 <- sum(!is.na(fixed2) & !fixed2, na.rm = TRUE)
        df <- abs(k2 - k1)
        has_lrt <- df > 0
      }
    }
  }

  # Check if pct_change is shown
  pct_cols <- grep("^pct_change(_\\d+)?$", names(comparison), value = TRUE)
  has_pct_change <- length(pct_cols) > 0 && any(!is.na(comparison[pct_cols]))

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
build_comparison_footnote <- function(
  comparison,
  n_sigfig,
  ofv_decimals = NULL
) {
  fallback_suffix_cols <- c(
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
  spec <- attr(comparison, "table_spec")
  suffix_cols <- get_comparison_suffix_cols(
    spec,
    comparison,
    fallback_suffix_cols,
    include_fixed_for_ci = TRUE
  )
  meta <- normalize_comparison_meta(comparison, suffix_cols)
  labels <- meta$labels
  summaries <- meta$summaries

  model_indices <- get_comparison_model_indices(names(comparison), suffix_cols)
  if (length(model_indices) < 2) {
    return(NULL)
  }

  if (length(labels) < length(model_indices)) {
    labels <- c(
      labels,
      paste0("Model ", (length(labels) + 1):length(model_indices))
    )
  }

  if (length(summaries) < length(model_indices)) {
    summaries <- c(
      summaries,
      rep(list(NULL), length(model_indices) - length(summaries))
    )
  }

  lines <- character(0)

  for (i in 2:length(model_indices)) {
    left_idx <- model_indices[i - 1]
    right_idx <- model_indices[i]
    left_label <- labels[i - 1]
    right_label <- labels[i]
    left_sum <- summaries[[i - 1]]
    right_sum <- summaries[[i]]

    cn1 <- if (!is.null(left_sum) && !is.null(left_sum$condition_number)) {
      left_sum$condition_number
    } else {
      NA
    }
    cn2 <- if (!is.null(right_sum) && !is.null(right_sum$condition_number)) {
      right_sum$condition_number
    } else {
      NA
    }

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
          left_label,
          cn2_str,
          right_label
        )
      )
    }

    nobs1 <- if (!is.null(left_sum) && !is.null(left_sum$number_obs)) {
      left_sum$number_obs
    } else {
      NA
    }
    nobs2 <- if (!is.null(right_sum) && !is.null(right_sum$number_obs)) {
      right_sum$number_obs
    } else {
      NA
    }

    if (!is.na(nobs1) || !is.na(nobs2)) {
      nobs1_str <- if (!is.na(nobs1)) as.character(nobs1) else "N/A"
      nobs2_str <- if (!is.na(nobs2)) as.character(nobs2) else "N/A"
      lines <- c(
        lines,
        sprintf(
          "No. of Observations: %s (%s), %s (%s)",
          nobs1_str,
          left_label,
          nobs2_str,
          right_label
        )
      )
    }

    ofv1 <- if (!is.null(left_sum) && !is.null(left_sum$ofv)) left_sum$ofv else
      NA
    ofv2 <- if (!is.null(right_sum) && !is.null(right_sum$ofv))
      right_sum$ofv else NA

    if (!is.na(ofv1) || !is.na(ofv2)) {
      ofv1_str <- if (!is.na(ofv1)) {
        format_hyperion_decimal_string(ofv1, ofv_decimals)
      } else {
        "-"
      }
      ofv2_str <- if (!is.na(ofv2)) {
        format_hyperion_decimal_string(ofv2, ofv_decimals)
      } else {
        "-"
      }

      ofv_parts <- c(
        sprintf(
          "OFV: %s (%s), %s (%s)",
          ofv1_str,
          left_label,
          ofv2_str,
          right_label
        )
      )

      if (!is.na(ofv1) && !is.na(ofv2)) {
        same_nobs <- !is.na(nobs1) && !is.na(nobs2) && nobs1 == nobs2
        if (same_nobs) {
          delta_ofv <- ofv2 - ofv1
          fixed1 <- comparison[[paste0("fixed_", left_idx)]]
          fixed2 <- comparison[[paste0("fixed_", right_idx)]]

          if (!is.null(fixed1) && !is.null(fixed2)) {
            k1 <- sum(!is.na(fixed1) & !fixed1, na.rm = TRUE)
            k2 <- sum(!is.na(fixed2) & !fixed2, na.rm = TRUE)
            df <- abs(k2 - k1)

            if (df > 0) {
              p_value <- stats::pchisq(abs(delta_ofv), df, lower.tail = FALSE)
              ofv_parts <- c(
                ofv_parts,
                sprintf(
                  "delta = %s, LRT p-value = %s (df=%d)",
                  format_hyperion_decimal_string(delta_ofv, ofv_decimals),
                  format(p_value, scientific = TRUE, digits = n_sigfig),
                  df
                )
              )
            }
          }
        }
      }

      lines <- c(lines, sprintf("%s", paste(ofv_parts, collapse = " | ")))
    }
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
  spec <- attr(comparison, "table_spec")
  n_sigfig <- if (!is.null(spec)) spec@n_sigfig else 3
  fallback_suffix_cols <- c(
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
  suffix_cols <- get_comparison_suffix_cols(
    spec,
    comparison,
    fallback_suffix_cols
  )
  meta <- normalize_comparison_meta(comparison, suffix_cols)
  labels <- meta$labels
  summaries <- meta$summaries
  model_indices <- get_comparison_model_indices(names(comparison), suffix_cols)

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
  attr(comparison, "summary1") <- summaries[[max(1, length(summaries) - 1)]]
  attr(comparison, "summary2") <- summaries[[length(summaries)]]
  attr(comparison, "summaries") <- summaries
  attr(comparison, "labels") <- labels

  display_cols <- get_comparison_suffix_cols(
    spec,
    comparison,
    fallback_suffix_cols
  )
  display_cols <- setdiff(display_cols, "pct_change")

  # Columns to hide (internal)
  hide_cols <- c("kind", "random_effect", "diagonal", ".appear_order")
  hide_suffix <- grep(
    "^(fixed|stderr|variability|shrinkage)_\\d+$",
    names(comparison),
    value = TRUE
  )
  if ("fixed" %in% display_cols) {
    hide_suffix <- hide_suffix[!grepl("^fixed_\\d+$", hide_suffix)]
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

  # Find columns that are all NA/empty (auto-hide these if enabled)
  if (!is.null(spec) && spec@hide_empty_columns) {
    empty_cols <- find_empty_columns(comparison)
    hide_cols <- unique(c(hide_cols, empty_cols))
  }

  pct_change_cols <- grep("^pct_change_\\d+$", names(comparison), value = TRUE)
  add_cols <- if (!is.null(spec)) spec@add_columns %||% character(0) else
    character(0)
  columns_provided <- !is.null(spec) && isTRUE(spec@columns_provided)
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

  # Apply drop_columns from spec to comparison-specific columns
  if (!is.null(spec) && length(spec@drop_columns) > 0) {
    drop_cols <- sub("_left$", "_1", spec@drop_columns)
    drop_cols <- sub("_right$", "_2", drop_cols)
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

  # CI merge - only if both bounds requested
  for (idx in model_indices) {
    ci_low <- paste0("ci_low_", idx)
    ci_high <- paste0("ci_high_", idx)
    fixed_col <- paste0("fixed_", idx)
    if (
      all(c(ci_low, ci_high, fixed_col) %in% names(comparison)) &&
        all(c("ci_low", "ci_high") %in% spec@columns)
    ) {
      rows_nonfixed <- !comparison[[fixed_col]] & !is.na(comparison[[ci_low]])
      rows_fixed <- comparison[[fixed_col]] & !is.na(comparison[[fixed_col]])
      table <- table |>
        gt::cols_merge(
          columns = c(ci_low, ci_high, fixed_col),
          rows = rows_nonfixed,
          pattern = "[{1}, {2}]"
        ) |>
        gt::cols_merge(
          columns = c(ci_low, ci_high, fixed_col),
          rows = rows_fixed,
          pattern = "Fixed"
        )
    }
  }

  # Hide internal columns
  hide_cols <- intersect(hide_cols, names(comparison))
  if (length(hide_cols) > 0) {
    table <- table |>
      gt::cols_hide(dplyr::all_of(hide_cols))
  }

  display_cols <- get_comparison_suffix_cols(
    spec,
    comparison,
    fallback_suffix_cols
  )
  if (all(c("ci_low", "ci_high") %in% display_cols)) {
    display_cols <- display_cols[display_cols != "ci_high"]
  }

  # Create spanners for each model
  model_cols <- list()
  for (idx in model_indices) {
    cols <- paste0(display_cols, "_", idx)
    cols <- intersect(cols, names(comparison))
    cols <- cols[!cols %in% hide_cols]
    if (all(c("ci_low", "ci_high") %in% spec@columns)) {
      cols <- cols[cols != paste0("ci_high_", idx)]
    }

    pct_col <- paste0("pct_change_", idx)
    if (pct_col %in% names(comparison) && !pct_col %in% hide_cols) {
      cols <- c(cols, pct_col)
    }

    model_cols[[as.character(idx)]] <- cols
  }

  # Reorder columns so pct_change sits after its corresponding model
  desired_cols <- c("name", unlist(model_cols, use.names = FALSE))
  remaining_cols <- setdiff(names(comparison), desired_cols)
  comparison <- comparison[, c(desired_cols, remaining_cols), drop = FALSE]
  attr(comparison, "summary1") <- summaries[[max(1, length(summaries) - 1)]]
  attr(comparison, "summary2") <- summaries[[length(summaries)]]
  attr(comparison, "summaries") <- summaries
  attr(comparison, "labels") <- labels

  for (i in seq_along(model_cols)) {
    cols <- model_cols[[i]]
    if (length(cols) > 0) {
      label <- if (length(labels) >= i) labels[i] else paste0("Model ", i)
      table <- table |>
        gt::tab_spanner(label = label, columns = dplyr::all_of(cols))
    }
  }

  # Rename columns for display
  pct_change_cols <- grep("^pct_change_\\d+$", names(comparison), value = TRUE)
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
  for (idx in model_indices) {
    for (col in names(base_labels)) {
      label <- base_labels[[col]]
      if (
        col == "ci_low" &&
          "ci_low" %in% spec@columns &&
          !"ci_high" %in% spec@columns
      ) {
        label <- sprintf("Lower %d%% CI", ci_pct)
      }
      if (
        col == "ci_high" &&
          "ci_high" %in% spec@columns &&
          !"ci_low" %in% spec@columns
      ) {
        label <- sprintf("Upper %d%% CI", ci_pct)
      }
      label_map[[paste0(col, "_", idx)]] <- label
    }
  }
  label_map <- label_map[setdiff(
    intersect(names(label_map), names(comparison)),
    hide_cols
  )]

  table <- table |>
    gt::cols_label(!!!label_map) |>
    gt::fmt_markdown() |>
    gt::fmt_number(
      columns = dplyr::any_of(c(
        paste0("estimate_", model_indices),
        paste0("rse_", model_indices),
        paste0("ci_low_", model_indices),
        paste0("ci_high_", model_indices),
        pct_change_cols,
        "pct_change"
      )),
      n_sigfig = n_sigfig
    ) |>
    gt::sub_missing(columns = dplyr::everything(), missing_text = "")

  # Add title if spec has one
  if (!is.null(spec) && !is.null(spec@title) && nchar(spec@title) > 0) {
    table <- table |>
      gt::tab_header(title = spec@title)
  }

  # Add comparison footnotes (each line is a separate footnote)
  ofv_decimals <- if (!is.null(spec) && !is.na(spec@n_decimals_ofv)) {
    spec@n_decimals_ofv
  } else {
    NULL
  }
  footnote_lines <- build_comparison_footnote(
    comparison,
    n_sigfig,
    ofv_decimals
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
  border_cols <- character(0)
  for (cols in model_cols) {
    if (length(cols) > 0) {
      border_cols <- c(border_cols, utils::tail(cols, 1))
    }
  }
  if (
    (length(pct_change_cols) > 0 || "pct_change" %in% names(comparison)) &&
      length(model_cols) > 0
  ) {
    last_cols <- model_cols[[length(model_cols)]]
    if (length(last_cols) > 0) {
      border_cols <- c(border_cols, utils::tail(last_cols, 1))
    }
  }

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
