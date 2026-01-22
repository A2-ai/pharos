# ==============================================================================
# Model comparison functions
# ==============================================================================

#' Capture all custom attributes from a comparison object
#'
#' Used to preserve attributes before dplyr operations which strip them.
#' @param comparison A hyperion_comparison object
#' @return Named list of attributes
#' @noRd
capture_comparison_attrs <- function(comparison) {
  list(
    summaries = attr(comparison, "summaries"),
    labels = attr(comparison, "labels"),
    table_spec = attr(comparison, "table_spec"),
    pct_change_refs = attr(comparison, "pct_change_refs"),
    lineage = attr(comparison, "lineage")
  )
}

#' Restore custom attributes to a comparison object
#'
#' Used to restore attributes after dplyr operations which strip them.
#' @param comparison A data frame to restore attributes to
#' @param attrs Named list of attributes from capture_comparison_attrs()
#' @return The comparison with attributes restored
#' @noRd
restore_comparison_attrs <- function(comparison, attrs) {
  for (name in names(attrs)) {
    if (!is.null(attrs[[name]])) {
      attr(comparison, name) <- attrs[[name]]
    }
  }
  comparison
}

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

  list(labels = labels, summaries = summaries)
}

#' @noRd
resolve_reference_context <- function(
  comparison,
  right_idx,
  model_indices,
  labels,
  summaries,
  fallback_pos
) {
  pct_change_refs <- attr(comparison, "pct_change_refs")
  pct_col <- paste0("pct_change_", right_idx)
  if (!is.null(pct_change_refs[[pct_col]])) {
    ref_idx <- pct_change_refs[[pct_col]]
    ref_pos <- which(model_indices == ref_idx)
    if (length(ref_pos) > 0) {
      return(list(
        left_idx = ref_idx,
        left_label = labels[ref_pos],
        left_sum = summaries[[ref_pos]]
      ))
    }
  }

  list(
    left_idx = model_indices[fallback_pos],
    left_label = labels[fallback_pos],
    left_sum = summaries[[fallback_pos]]
  )
}

#' @noRd
can_show_lrt <- function(comparison, left_idx, right_idx, left_sum, right_sum) {
  lineage <- attr(comparison, "lineage")
  if (is.null(lineage)) {
    return(FALSE)
  }
  if (is.null(left_sum) || is.null(right_sum)) {
    return(FALSE)
  }

  ofv1 <- if (!is.null(left_sum$ofv)) left_sum$ofv else NA
  ofv2 <- if (!is.null(right_sum$ofv)) right_sum$ofv else NA
  if (is.na(ofv1) || is.na(ofv2)) {
    return(FALSE)
  }

  nobs1 <- if (!is.null(left_sum$number_obs)) left_sum$number_obs else NA
  nobs2 <- if (!is.null(right_sum$number_obs)) right_sum$number_obs else NA
  if (is.na(nobs1) || is.na(nobs2) || nobs1 != nobs2) {
    return(FALSE)
  }

  fixed1 <- comparison[[paste0("fixed_", left_idx)]]
  fixed2 <- comparison[[paste0("fixed_", right_idx)]]
  if (is.null(fixed1) || is.null(fixed2)) {
    return(FALSE)
  }
  k1 <- sum(!is.na(fixed1) & !fixed1, na.rm = TRUE)
  k2 <- sum(!is.na(fixed2) & !fixed2, na.rm = TRUE)
  df <- abs(k2 - k1)
  if (df <= 0) {
    return(FALSE)
  }

  run_name1 <- if (!is.null(left_sum$run_name)) left_sum$run_name else NULL
  run_name2 <- if (!is.null(right_sum$run_name)) right_sum$run_name else NULL
  if (is.null(run_name1) || is.null(run_name2)) {
    return(FALSE)
  }

  are_models_in_lineage(lineage, run_name1, run_name2)
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
#' @param reference_model Character string specifying which model to use as
#'   reference for percent change calculations. Should match the `run_name` of
#'   one of the models already in the comparison. When NULL (default), percent
#'   change is calculated relative to the previous model in the chain.
#'
#' @return Data frame with class `hyperion_comparison` containing joined
#'   parameter data with suffixed columns and comparison attributes.
#'
#' @export
compare_with <- function(
  params1,
  params2,
  labels = c("Model 1", "Model 2"),
  reference_model = NULL
) {
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
  columns_provided <- !is.null(spec1) && isTRUE(spec1@.columns_provided)
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

  if (!is_comparison && !is.null(reference_model)) {
    warning("reference_model is ignored for initial two-model comparisons")
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

  # Calculate percent change: (estN - estRef) / estRef * 100
  if (is_comparison) {
    last_idx <- next_index
    prev_idx <- if (length(model_indices) > 0) max(model_indices) else
      next_index - 1
  } else {
    last_idx <- 2
    prev_idx <- 1
  }

  # Handle reference_model parameter for percent change calculation
  ref_idx <- prev_idx
  if (!is.null(reference_model) && is_comparison) {
    # Normalize reference_model (strip .mod if present)
    ref_model_clean <- sub("\\.mod$", "", reference_model)
    found <- FALSE
    # First try matching by run_name in summaries
    for (i in seq_along(existing_summaries)) {
      sum_i <- existing_summaries[[i]]
      if (!is.null(sum_i) && !is.null(sum_i$run_name)) {
        # Normalize run_name for comparison
        run_name_clean <- sub("\\.mod$", "", sum_i$run_name)
        if (run_name_clean == ref_model_clean) {
          ref_idx <- model_indices[i]
          found <- TRUE
          break
        }
      }
    }
    # Fall back to matching by label if run_name didn't match
    if (!found) {
      for (i in seq_along(existing_labels)) {
        label_clean <- sub("\\.mod$", "", existing_labels[i])
        if (label_clean == ref_model_clean) {
          ref_idx <- model_indices[i]
          break
        }
      }
    }
  }

  est_ref <- paste0("estimate_", ref_idx)
  est_last <- paste0("estimate_", last_idx)
  pct_col <- paste0("pct_change_", last_idx)
  if (est_ref %in% names(comparison) && est_last %in% names(comparison)) {
    comparison[[pct_col]] <- dplyr::case_when(
      is.na(comparison[[est_ref]]) | is.na(comparison[[est_last]]) ~ NA_real_,
      comparison[[est_ref]] == 0 ~ NA_real_,
      TRUE ~
        (comparison[[est_last]] - comparison[[est_ref]]) /
          comparison[[est_ref]] *
          100
    )
    comparison$pct_change <- comparison[[pct_col]]
  }

  # Track pct_change reference indices (which model each pct_change compares to)
  existing_pct_refs <- attr(params1, "pct_change_refs")
  if (is.null(existing_pct_refs)) {
    existing_pct_refs <- list()
  }
  existing_pct_refs[[pct_col]] <- ref_idx

  # Attach class and attributes
  class(comparison) <- c("hyperion_comparison", class(comparison))
  if (is_comparison) {
    summaries <- c(existing_summaries, list(sum2))
  } else {
    summaries <- list(sum1, sum2)
  }
  attr(comparison, "summaries") <- summaries
  attr(comparison, "labels") <- labels
  attr(comparison, "table_spec") <- spec
  attr(comparison, "pct_change_refs") <- existing_pct_refs

  comparison
}

#' Add model lineage to a comparison object
#'
#' Attaches lineage information to a comparison object to enable lineage-aware
#' features like conditional LRT display. When lineage is attached, the LRT
#' footnote will only be shown for model pairs that are in a direct
#' ancestor-descendant relationship.
#'
#' @param comparison A hyperion_comparison object from `compare_with()`
#' @param lineage A hyperion_nonmem_tree object from `get_model_lineage()`
#'
#' @return The comparison object with lineage attribute attached
#'
#' @export
add_model_lineage <- function(comparison, lineage) {
  if (!inherits(comparison, "hyperion_comparison")) {
    stop("comparison must be a hyperion_comparison object from compare_with()")
  }
  if (!inherits(lineage, "hyperion_nonmem_tree")) {
    stop(
      "lineage must be a hyperion_nonmem_tree object from get_model_lineage()"
    )
  }

  attr(comparison, "lineage") <- lineage
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
  meta <- normalize_comparison_meta(comparison, suffix_cols)
  summaries <- meta$summaries
  labels <- meta$labels
  model_indices <- get_comparison_model_indices(names(comparison), suffix_cols)

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

  # Get the last model and its reference
  ref_idx <- NULL
  if (length(model_indices) > 1) {
    last_idx <- utils::tail(model_indices, 1)
    right_pos <- length(model_indices)
    ref_ctx <- resolve_reference_context(
      comparison,
      last_idx,
      model_indices,
      labels,
      summaries,
      fallback_pos = right_pos - 1
    )
    sum1 <- ref_ctx$left_sum
    sum2 <- summaries[[right_pos]]
    ref_idx <- ref_ctx$left_idx
  } else {
    sum1 <- if (length(summaries) >= 1) summaries[[1]] else NULL
    sum2 <- if (length(summaries) >= 2) summaries[[2]] else NULL
    ref_idx <- 1
  }

  # Check if OFV is shown
  has_ofv <- length(summaries) > 0 &&
    any(vapply(
      summaries,
      function(sum) !is.null(sum$ofv) && !is.na(sum$ofv),
      logical(1)
    ))

  # Check if LRT is shown (both OFVs, same nobs, df > 0)
  has_lrt <- FALSE
  if (length(model_indices) > 1) {
    for (i in seq(2, length(model_indices))) {
      right_idx <- model_indices[i]
      right_sum <- summaries[[i]]
      ref_ctx <- resolve_reference_context(
        comparison,
        right_idx,
        model_indices,
        meta$labels,
        summaries,
        fallback_pos = i - 1
      )
      if (
        can_show_lrt(
          comparison,
          ref_ctx$left_idx,
          right_idx,
          ref_ctx$left_sum,
          right_sum
        )
      ) {
        has_lrt <- TRUE
        break
      }
    }
  }

  # Check if pct_change is shown
  pct_cols <- grep("^pct_change(_\\d+)?$", names(comparison), value = TRUE)
  has_pct_change <- length(pct_cols) > 0 && any(!is.na(comparison[pct_cols]))

  list(
    has_ofv = has_ofv,
    has_lrt = has_lrt,
    has_pct_change = has_pct_change,
    ref_idx = ref_idx
  )
}

#' Build comparison footnote with OFV and LRT statistics
#'
#' @param comparison Data frame from compare_with()
#' @param n_sigfig Number of significant figures for formatting
#' @param ofv_decimals Number of decimal places for OFV values
#' @param pvalue_scientific If TRUE, format p-values in scientific notation
#' @param pvalue_threshold If not NULL, p-values below this show as "< threshold"
#' @return Character vector of footnote lines, or NULL if no summaries
#' @noRd
build_comparison_footnote <- function(
  comparison,
  n_sigfig,
  ofv_decimals = NULL,
  pvalue_scientific = TRUE,
  pvalue_threshold = NULL
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
    right_idx <- model_indices[i]
    right_label <- labels[i]
    right_sum <- summaries[[i]]

    ref_ctx <- resolve_reference_context(
      comparison,
      right_idx,
      model_indices,
      labels,
      summaries,
      fallback_pos = i - 1
    )
    left_idx <- ref_ctx$left_idx
    left_label <- ref_ctx$left_label
    left_sum <- ref_ctx$left_sum

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
              if (
                can_show_lrt(
                  comparison,
                  left_idx,
                  right_idx,
                  left_sum,
                  right_sum
                )
              ) {
                p_value <- lrt_pvalue(abs(delta_ofv), df)
                pval_str <- format_pvalue_string(
                  p_value,
                  n_sigfig,
                  pvalue_scientific,
                  pvalue_threshold
                )
                ofv_parts <- c(
                  ofv_parts,
                  sprintf(
                    "delta = %s, LRT p-value = %s (df=%d)",
                    format_hyperion_decimal_string(delta_ofv, ofv_decimals),
                    pval_str,
                    df
                  )
                )
              }
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
  if (is.null(spec)) {
    stop("TableSpec not found. Run apply_table_spec(params, spec, info) first.")
  }
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
  # Prepare data + layout (sections, fixed display, hide rules, labels).
  prep <- prepare_comparison_table_data(
    comparison,
    spec,
    fallback_suffix_cols
  )
  comparison <- prep$comparison
  layout <- prep$layout
  labels <- prep$labels
  summaries <- prep$summaries
  suffix_cols <- prep$suffix_cols
  model_indices <- prep$model_indices

  display_cols <- layout$display_cols
  hide_cols <- layout$hide_cols
  show_pct_change <- layout$show_pct_change
  pct_change_cols <- layout$pct_change_cols
  fixed_display_cols <- layout$fixed_display_cols
  groupname <- layout$groupname

  # Build gt table shell.
  table <- comparison |>
    gt::gt(groupname_col = groupname)

  ci_pct <- get_ci_pct(spec, default = 95)

  # Merge CI bounds per model when requested.
  table <- apply_comparison_ci_merge(table, comparison, spec, model_indices)

  # Hide internal/unused columns.
  hide_cols <- intersect(hide_cols, names(comparison))
  if (length(hide_cols) > 0) {
    table <- table |>
      gt::cols_hide(dplyr::all_of(hide_cols))
  }

  # Compute model spanner columns and reorder the data.
  # Capture attrs before reordering (which strips custom attributes)
  saved_attrs <- capture_comparison_attrs(comparison)
  model_layout <- compute_comparison_model_cols(
    comparison,
    display_cols,
    model_indices,
    hide_cols,
    spec,
    show_pct_change
  )
  comparison <- model_layout$comparison
  model_cols <- model_layout$model_cols
  comparison <- restore_comparison_attrs(comparison, saved_attrs)

  # Apply model spanners.
  table <- apply_model_spanners(table, model_cols, labels)

  # Build display labels and apply numeric formatting.
  label_map <- build_comparison_label_map(
    labels,
    pct_change_cols,
    show_pct_change,
    ci_pct,
    spec,
    fixed_display_cols,
    model_indices,
    comparison,
    hide_cols
  )

  table <- apply_standard_gt_formatting(
    table,
    label_map,
    n_sigfig,
    c(
      paste0("estimate_", model_indices),
      paste0("rse_", model_indices),
      paste0("ci_low_", model_indices),
      paste0("ci_high_", model_indices),
      pct_change_cols,
      "pct_change"
    )
  )

  # Title + footnotes.
  table <- apply_table_title(table, spec@title)

  table <- apply_comparison_footnotes(table, comparison, spec, n_sigfig, ci_pct)

  # Style: bold headers
  # Final styling (bold headers + borders + nowrap).
  table <- apply_standard_gt_styling(
    table,
    include_row_groups = TRUE,
    include_spanners = TRUE
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

  table
}
