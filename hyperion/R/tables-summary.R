# ==============================================================================
# User-facing DSL functions
# ==============================================================================

#' Create filter rules for model selection
#'
#' Creates rules for filtering which models appear in the summary table.
#' Rules are evaluated against model metadata from the lineage tree.
#'
#' @param ... Filter expressions like `"final" %in% tags`, `!is.null(description)`
#'
#' @section Available columns:
#' The following columns are available for use in filter rules:
#' \itemize{
#'   \item `name` - Model filename (e.g., "run001.mod")
#'   \item `description` - Model description from metadata
#'   \item `tags` - Character vector of tags
#'   \item `based_on` - Character vector of parent model(s)
#' }
#'
#' @return List of quosures for use in SummarySpec
#' @examples
#' summary_filter_rules(
#'   "production" %in% tags,
#'   !is.null(description),
#'   name %in% c("run001.mod", "run003.mod")
#' )
#' @export
summary_filter_rules <- function(...) {
  rlang::enquos(...)
}

# ==============================================================================
# SummarySpec S7 Class
# ==============================================================================

#' Summary specification for run summary tables
#'
#' @param model_filter List of filter rules created with `summary_filter_rules()`
#' @param fields Character vector of fields to include. Valid fields:
#'   "based_on", "description", "n_parameters", "problem",
#'   "number_data_records", "number_subjects", "number_obs",
#'   "estimation_method", "estimation_time", "covariance_time",
#'   "postprocess_time", "function_evaluations", "significant_digits",
#'   "ofv", "dofv", "condition_number", "termination_status", "pvalue", "df".
#'   Note: "pvalue" and "df" require "dofv" to be calculated; pvalue uses the
#'   Likelihood Ratio Test (LRT) assuming nested models.
#' @param drop_columns Character vector of columns to exclude from output, or
#'   NULL (default).
#' @param n_sigfig Number of significant figures for numeric formatting.
#'   Default is 3.
#' @param n_decimals_ofv Number of decimal places for OFV and dOFV values.
#'   Default is 3.
#' @param time_format Format for time columns. Options: "seconds" (default),
#'   "minutes", "hours", "auto" (auto-scale based on magnitude).
#' @param title Character. Title for the table header. Default is
#'   "Run Summary".
#' @param hide_empty_columns Logical. If TRUE, columns with all NA values are
#'   hidden. Default is TRUE.
#' @param remove_unrun_models Logical. If TRUE (default), models without
#'   completed runs are excluded from the table.
#' @param tag_filter Character vector of tags, or NULL (default). Only models
#'   with at least one matching tag are included.
#' @param pvalue_scientific Logical. If TRUE (default), p-values are formatted
#'   in scientific notation (e.g., 1.23e-04). If FALSE, uses significant figures
#'   from n_sigfig.
#'
#' @export
SummarySpec <- S7::new_class(
  "SummarySpec",
  properties = list(
    model_filter = S7::new_property(
      class = S7::class_list,
      default = list()
    ),
    fields = S7::new_property(
      class = S7::class_character,
      default = c(
        "based_on",
        "description",
        "n_parameters",
        "condition_number",
        "ofv",
        "dofv",
        "pvalue"
      )
    ),
    drop_columns = S7::new_property(
      class = S7::class_character | NULL,
      default = NULL
    ),
    n_sigfig = S7::new_property(
      class = S7::class_numeric,
      default = 3
    ),
    n_decimals_ofv = S7::new_property(
      class = S7::class_numeric,
      default = 3
    ),
    time_format = S7::new_property(
      class = S7::class_character,
      default = "seconds"
    ),
    title = S7::new_property(
      class = S7::class_character,
      default = "Run Summary"
    ),
    hide_empty_columns = S7::new_property(
      class = S7::class_logical,
      default = TRUE
    ),
    remove_unrun_models = S7::new_property(
      class = S7::class_logical,
      default = TRUE
    ),
    tag_filter = S7::new_property(
      class = S7::class_character | NULL,
      default = NULL
    ),
    fields_provided = S7::new_property(
      class = S7::class_logical,
      default = FALSE
    ),
    pvalue_scientific = S7::new_property(
      class = S7::class_logical,
      default = TRUE
    )
  ),
  validator = function(self) {
    valid_fields <- c(
      # From tree metadata
      "based_on",
      "description",
      # From parameters
      "n_parameters",
      # From run_details
      "problem",
      "number_data_records",
      "number_subjects",
      "number_obs",
      "estimation_method",
      "estimation_time",
      "covariance_time",
      "postprocess_time",
      "function_evaluations",
      "significant_digits",
      # From minimization_results
      "ofv",
      "dofv",
      "condition_number",
      "termination_status",
      # Computed LRT fields
      "pvalue",
      "df"
    )

    if (!all(self@fields %in% valid_fields)) {
      bad <- setdiff(self@fields, valid_fields)
      return(sprintf(
        "@fields must be in: %s\n  Got: %s",
        paste(valid_fields, collapse = ", "),
        paste(bad, collapse = ", ")
      ))
    }

    if (!self@time_format %in% c("seconds", "minutes", "hours", "auto")) {
      return(sprintf(
        "@time_format must be 'seconds', 'minutes', 'hours', or 'auto'. Got: '%s'",
        self@time_format
      ))
    }

    if (
      length(self@n_sigfig) != 1 ||
        self@n_sigfig < 1 ||
        self@n_sigfig != floor(self@n_sigfig)
    ) {
      return(sprintf(
        "@n_sigfig must be a positive whole number. Got: %s",
        self@n_sigfig
      ))
    }

    if (
      length(self@hide_empty_columns) != 1 || is.na(self@hide_empty_columns)
    ) {
      return("@hide_empty_columns must be TRUE or FALSE")
    }

    if (
      length(self@remove_unrun_models) != 1 || is.na(self@remove_unrun_models)
    ) {
      return("@remove_unrun_models must be TRUE or FALSE")
    }

    if (
      length(self@model_filter) > 0 &&
        !all(vapply(self@model_filter, rlang::is_quosure, logical(1)))
    ) {
      return("@model_filter rules must be created with summary_filter_rules()")
    }

    if (
      !is.null(self@drop_columns) && !all(self@drop_columns %in% valid_fields)
    ) {
      bad <- setdiff(self@drop_columns, valid_fields)
      return(sprintf(
        "@drop_columns must be in: %s\n  Got: %s",
        paste(valid_fields, collapse = ", "),
        paste(bad, collapse = ", ")
      ))
    }

    if (length(self@fields_provided) != 1 || is.na(self@fields_provided)) {
      return(sprintf(
        "@fields_provided must be TRUE or FALSE. Got: %s",
        self@fields_provided
      ))
    }

    if (length(self@pvalue_scientific) != 1 || is.na(self@pvalue_scientific)) {
      return(sprintf(
        "@pvalue_scientific must be TRUE or FALSE. Got: %s",
        self@pvalue_scientific
      ))
    }
  },
  constructor = function(
    model_filter = list(),
    fields = NULL,
    drop_columns = NULL,
    n_sigfig = 3,
    n_decimals_ofv = 3,
    time_format = "seconds",
    title = "Run Summary",
    hide_empty_columns = TRUE,
    remove_unrun_models = TRUE,
    tag_filter = NULL,
    pvalue_scientific = TRUE
  ) {
    fields_provided <- !is.null(fields)

    if (is.null(fields)) {
      fields <- c(
        "based_on",
        "description",
        "n_parameters",
        "condition_number",
        "ofv",
        "dofv",
        "pvalue"
      )
    }

    S7::new_object(
      S7::S7_object(),
      model_filter = model_filter,
      fields = fields,
      drop_columns = drop_columns,
      n_sigfig = n_sigfig,
      n_decimals_ofv = n_decimals_ofv,
      time_format = time_format,
      title = title,
      hide_empty_columns = hide_empty_columns,
      remove_unrun_models = remove_unrun_models,
      tag_filter = tag_filter,
      fields_provided = fields_provided,
      pvalue_scientific = pvalue_scientific
    )
  }
)

# ==============================================================================
# Apply spec to lineage tree
# ==============================================================================

#' Apply summary specification to lineage tree
#'
#' Filters models, loads summaries, and prepares data for table generation.
#'
#' @param tree A hyperion_nonmem_tree object from `get_model_lineage()`
#' @param spec A SummarySpec object
#'
#' @return Data frame with summary_spec attribute, ready for make_summary_table()
#' @export
apply_summary_spec <- function(tree, spec = SummarySpec()) {
  if (!requireNamespace("dplyr", quietly = TRUE)) {
    stop("Package 'dplyr' is required for apply_summary_spec()")
  }
  if (!inherits(tree, "hyperion_nonmem_tree")) {
    stop("tree must be a hyperion_nonmem_tree object from get_model_lineage()")
  }
  if (!S7::S7_inherits(spec, SummarySpec)) {
    stop("spec must be a SummarySpec object")
  }

  source_dir <- tree$source_dir
  if (is.null(source_dir) || source_dir == "") {
    stop(
      "Tree does not have source_dir. ",
      "Ensure you are using a recent version of get_model_lineage()."
    )
  }

  # Extract model metadata for filtering
  model_names <- names(tree$nodes)
  if (length(model_names) == 0) {
    return(build_empty_summary_df(spec))
  }

  # Build metadata data frame for filtering
  metadata_df <- build_metadata_df(tree)

  # Apply tag_filter - keep models that have any of the specified tags
  if (!is.null(spec@tag_filter)) {
    has_matching_tag <- vapply(
      metadata_df$tags,
      function(model_tags) {
        any(spec@tag_filter %in% model_tags)
      },
      logical(1)
    )
    metadata_df <- metadata_df[has_matching_tag, , drop = FALSE]
  }

  # Apply model_filter rules
  if (length(spec@model_filter) > 0) {
    for (f in spec@model_filter) {
      metadata_df <- metadata_df |>
        dplyr::filter(!!f)
    }
  }

  if (nrow(metadata_df) == 0) {
    return(build_empty_summary_df(spec))
  }

  # Topologically sort models
  sorted_names <- topological_sort_models(metadata_df, tree)

  # Load summaries and extract run details
  summaries <- load_model_summaries(sorted_names, source_dir)

  # Build summary data frame (pass metadata for based_on/description)
  df <- build_summary_df(summaries, sorted_names, metadata_df, spec)

  # Apply drop_columns
  if (!is.null(spec@drop_columns)) {
    df <- df[, setdiff(names(df), spec@drop_columns), drop = FALSE]
  }

  attr(df, "summary_spec") <- spec
  df
}

# ==============================================================================
# Helper functions
# ==============================================================================

#' Build metadata data frame from tree nodes
#' @noRd
build_metadata_df <- function(tree) {
  nodes <- tree$nodes

  rows <- lapply(names(nodes), function(name) {
    node <- nodes[[name]]
    data.frame(
      name = name,
      description = node$description %||% NA_character_,
      tags = I(list(node$tags %||% character(0))),
      based_on = I(list(node$based_on %||% character(0))),
      stringsAsFactors = FALSE
    )
  })

  dplyr::bind_rows(rows)
}

#' Build empty summary data frame
#' @noRd
build_empty_summary_df <- function(spec) {
  df <- data.frame(model = character(0), stringsAsFactors = FALSE)
  char_fields <- c(
    "based_on",
    "description",
    "problem",
    "estimation_method",
    "termination_status"
  )
  for (field in spec@fields) {
    df[[field]] <- vector(
      mode = if (field %in% char_fields) "character" else "numeric",
      length = 0
    )
  }
  attr(df, "summary_spec") <- spec
  df
}

#' Topological sort of models based on based_on relationships
#' @noRd
topological_sort_models <- function(metadata_df, tree) {
  names_to_sort <- metadata_df$name

  # Build adjacency: parent -> children
  # A model's parents are in based_on
  parent_of <- list()
  for (i in seq_len(nrow(metadata_df))) {
    name <- metadata_df$name[i]
    based_on <- metadata_df$based_on[[i]]
    parent_of[[name]] <- based_on
  }

  # Kahn's algorithm for topological sort
  # Count incoming edges (parents)
  in_degree <- integer(length(names_to_sort))
  names(in_degree) <- names_to_sort
  for (name in names_to_sort) {
    parents <- parent_of[[name]]
    # Only count parents that are in our filtered set
    in_degree[name] <- sum(parents %in% names_to_sort)
  }

  # Start with nodes that have no parents (in our filtered set)
  queue <- names_to_sort[in_degree == 0]
  sorted <- character(0)

  while (length(queue) > 0) {
    current <- queue[1]
    queue <- queue[-1]
    sorted <- c(sorted, current)

    # Find children of current (models where current is in based_on)
    for (name in names_to_sort) {
      if (name %in% sorted) next
      if (current %in% parent_of[[name]]) {
        in_degree[name] <- in_degree[name] - 1
        if (in_degree[name] == 0) {
          queue <- c(queue, name)
        }
      }
    }
  }

  # If not all nodes sorted, there's a cycle - just append remaining
  remaining <- setdiff(names_to_sort, sorted)
  c(sorted, remaining)
}

#' Load model summaries for given model names
#' @noRd
load_model_summaries <- function(model_names, source_dir) {
  summaries <- list()
  for (name in model_names) {
    # Strip .mod extension to get the output directory name
    run_name <- sub("\\.mod$", "", name)
    model_path <- file.path(source_dir, run_name)

    # Skip if output directory doesn't exist (unrun model)
    if (!dir.exists(model_path)) {
      summaries[[name]] <- NULL
      next
    }

    tryCatch(
      {
        summaries[[name]] <- get_model_summary(model_path)
      },
      error = function(e) {
        # Silently set to NULL - unrun models handled by remove_unrun_models
        summaries[[name]] <<- NULL
      }
    )
  }
  summaries
}

#' Build summary data frame from loaded summaries
#' @noRd
build_summary_df <- function(summaries, model_names, metadata_df, spec) {
  # Check if comparison stats are requested - need ofv, number_obs, n_parameters
  needs_dofv <- any(c("dofv", "pvalue", "df") %in% spec@fields)

  rows <- lapply(model_names, function(name) {
    mod_sum <- summaries[[name]]
    meta_row <- metadata_df[metadata_df$name == name, ]

    if (is.null(mod_sum)) {
      row <- build_na_row(name, meta_row, spec, needs_dofv)
      row$.unrun <- TRUE
      return(row)
    }

    # Extract last row from run_details
    rd <- mod_sum$run_details
    mr <- mod_sum$minimization_results

    if (is.null(rd) || nrow(rd) == 0) {
      row <- build_na_row(name, meta_row, spec, needs_dofv)
      row$.unrun <- TRUE
      return(row)
    }

    last_idx <- nrow(rd)

    row <- data.frame(
      model = mod_sum$run_name %||% sub("\\.mod$", "", name),
      .unrun = FALSE,
      stringsAsFactors = FALSE
    )

    # Store internal name for dofv lookup
    row$.name <- name

    # Extract metadata fields (based_on, description)
    if ("based_on" %in% spec@fields || needs_dofv) {
      parents <- if (nrow(meta_row) > 0) meta_row$based_on[[1]] else
        character(0)
      if (length(parents) > 0) {
        # Format based_on: strip .mod and join with comma
        row$based_on <- paste(sub("\\.mod$", "", parents), collapse = ", ")
        # Store raw based_on for dofv lookup (first parent only)
        row$.based_on_raw <- as.character(parents[1])
      } else {
        row$based_on <- NA_character_
        row$.based_on_raw <- NA_character_
      }
    }

    if ("description" %in% spec@fields) {
      if (nrow(meta_row) > 0 && !is.na(meta_row$description)) {
        row$description <- meta_row$description
      } else {
        row$description <- NA_character_
      }
    }

    # Extract n_parameters (count of non-fixed parameters)
    # Always extract if needs_dofv for pvalue/df calculation
    if ("n_parameters" %in% spec@fields || needs_dofv) {
      if (
        !is.null(mod_sum$parameters) &&
          nrow(mod_sum$parameters) > 0 &&
          "fixed" %in% names(mod_sum$parameters)
      ) {
        row$n_parameters <- sum(!mod_sum$parameters$fixed, na.rm = TRUE)
      } else {
        row$n_parameters <- NA_integer_
      }
    }

    # Extract fields from run_details
    rd_fields <- c(
      "problem",
      "number_data_records",
      "number_subjects",
      "number_obs",
      "estimation_method",
      "estimation_time",
      "covariance_time",
      "postprocess_time",
      "function_evaluations",
      "significant_digits"
    )

    # Always extract number_obs if we need dofv
    fields_to_extract <- if (needs_dofv) {
      unique(c(intersect(spec@fields, rd_fields), "number_obs"))
    } else {
      intersect(spec@fields, rd_fields)
    }

    for (field in fields_to_extract) {
      if (field %in% names(rd)) {
        row[[field]] <- rd[[field]][last_idx]
      } else {
        row[[field]] <- NA
      }
    }

    # Extract fields from minimization_results
    mr_fields <- c("ofv", "condition_number", "termination_status")

    # Always extract ofv if we need dofv
    mr_fields_to_extract <- if (needs_dofv) {
      unique(c(intersect(spec@fields, mr_fields), "ofv"))
    } else {
      intersect(spec@fields, mr_fields)
    }

    if (!is.null(mr) && nrow(mr) > 0) {
      last_mr_idx <- min(last_idx, nrow(mr))
      for (field in mr_fields_to_extract) {
        if (field %in% names(mr)) {
          row[[field]] <- mr[[field]][last_mr_idx]
        } else {
          row[[field]] <- NA
        }
      }
    } else {
      for (field in mr_fields_to_extract) {
        row[[field]] <- NA
      }
    }

    row
  })

  df <- dplyr::bind_rows(rows)

  # Remove unrun models if requested
  if (spec@remove_unrun_models && ".unrun" %in% names(df)) {
    df <- df[!df$.unrun, , drop = FALSE]
    df <- df[, setdiff(names(df), ".unrun"), drop = FALSE]
  } else if (".unrun" %in% names(df)) {
    df <- df[, setdiff(names(df), ".unrun"), drop = FALSE]
  }

  # Calculate dofv, df, pvalue if requested
  if (needs_dofv) {
    df <- calculate_dofv(df, summaries, spec)
  }

  # Apply time formatting
  df <- format_time_columns(df, spec)

  # Remove internal columns if not needed in output
  internal_cols <- c(".name", ".based_on_raw")
  if (!needs_dofv && "based_on" %in% spec@fields) {
    # Keep based_on but remove internals
    df <- df[, setdiff(names(df), internal_cols), drop = FALSE]
  } else if (needs_dofv) {
    # Remove .name but keep data for now
    df <- df[, setdiff(names(df), ".name"), drop = FALSE]
    # Remove .based_on_raw too
    df <- df[, setdiff(names(df), ".based_on_raw"), drop = FALSE]
  }

  # Remove internal columns that weren't requested (but we needed them for calculations)
  # Keep df if pvalue is requested (needed for merge in table)
  keep_df_for_pvalue <- "pvalue" %in% spec@fields
  internal_calc_cols <- c(
    "number_obs",
    "ofv",
    "n_parameters",
    "dofv",
    "df",
    "pvalue"
  )
  for (col in internal_calc_cols) {
    if (col == "df" && keep_df_for_pvalue) next
    if (needs_dofv && col %in% names(df) && !col %in% spec@fields) {
      df <- df[, setdiff(names(df), col), drop = FALSE]
    }
  }

  # Reorder columns to match spec@fields order
  # Include df after pvalue if pvalue is requested (for merge)
  col_order <- c("model", intersect(spec@fields, names(df)))
  if (keep_df_for_pvalue && "df" %in% names(df) && !"df" %in% col_order) {
    pvalue_idx <- which(col_order == "pvalue")
    if (length(pvalue_idx) > 0) {
      col_order <- append(col_order, "df", after = pvalue_idx)
    }
  }
  df <- df[, col_order, drop = FALSE]

  df
}

#' Calculate dOFV, df, and p-value for each model vs its parent
#' @noRd
calculate_dofv <- function(df, summaries, spec) {
  # Create lookups by .name
  ofv_lookup <- stats::setNames(df$ofv, df$.name)
  nobs_lookup <- stats::setNames(df$number_obs, df$.name)
  npar_lookup <- stats::setNames(df$n_parameters, df$.name)

  dofv_excluded <- FALSE
  needs_pvalue <- "pvalue" %in% spec@fields || "df" %in% spec@fields

  results <- lapply(seq_len(nrow(df)), function(i) {
    parent_name <- df$.based_on_raw[i]

    # No parent - no comparison stats
    if (is.na(parent_name) || parent_name == "") {
      return(list(dofv = NA_real_, df = NA_integer_, pvalue = NA_real_))
    }

    # Check if parent is in our data
    if (!parent_name %in% names(ofv_lookup)) {
      return(list(dofv = NA_real_, df = NA_integer_, pvalue = NA_real_))
    }

    parent_ofv <- ofv_lookup[[parent_name]]
    parent_nobs <- nobs_lookup[[parent_name]]
    parent_npar <- npar_lookup[[parent_name]]
    model_ofv <- df$ofv[i]
    model_nobs <- df$number_obs[i]
    model_npar <- df$n_parameters[i]

    # Check if number_obs matches
    if (is.na(model_nobs) || is.na(parent_nobs) || model_nobs != parent_nobs) {
      dofv_excluded <<- TRUE
      return(list(dofv = NA_real_, df = NA_integer_, pvalue = NA_real_))
    }

    # Calculate dofv
    if (is.na(model_ofv) || is.na(parent_ofv)) {
      return(list(dofv = NA_real_, df = NA_integer_, pvalue = NA_real_))
    }

    dofv <- model_ofv - parent_ofv

    # Calculate df and p-value if requested
    df_val <- NA_integer_
    pvalue <- NA_real_

    if (needs_pvalue && !is.na(model_npar) && !is.na(parent_npar)) {
      df_calc <- as.integer(model_npar - parent_npar)
      # Only calculate p-value if df > 0 (more complex model)
      if (!is.na(df_calc) && df_calc > 0) {
        df_val <- df_calc
        pvalue <- lrt_pvalue(-dofv, df_val)
      }
    }

    list(dofv = dofv, df = df_val, pvalue = pvalue)
  })

  df$dofv <- vapply(results, function(x) x$dofv, numeric(1))
  df$df <- vapply(results, function(x) x$df, integer(1))
  df$pvalue <- vapply(results, function(x) x$pvalue, numeric(1))

  attr(df, "dofv_excluded_nobs_mismatch") <- dofv_excluded
  df
}

#' Build a row with NA values for a model
#' @noRd
build_na_row <- function(name, meta_row, spec, needs_dofv = FALSE) {
  row <- data.frame(model = sub("\\.mod$", "", name), stringsAsFactors = FALSE)

  # Add internal columns for dofv lookup
  row$.name <- name
  if (needs_dofv || "based_on" %in% spec@fields) {
    parents <- if (nrow(meta_row) > 0) meta_row$based_on[[1]] else character(0)
    if (length(parents) > 0) {
      row$based_on <- paste(sub("\\.mod$", "", parents), collapse = ", ")
      row$.based_on_raw <- as.character(parents[1])
    } else {
      row$based_on <- NA_character_
      row$.based_on_raw <- NA_character_
    }
  }

  for (field in spec@fields) {
    if (field == "based_on") {
      # Already handled above
      next
    } else if (field == "description") {
      if (nrow(meta_row) > 0 && !is.na(meta_row$description)) {
        row$description <- meta_row$description
      } else {
        row$description <- NA_character_
      }
    } else {
      row[[field]] <- NA
    }
  }

  # Add number_obs and ofv for dofv calculation if needed
  if (needs_dofv) {
    if (!"number_obs" %in% spec@fields) row$number_obs <- NA_real_
    if (!"ofv" %in% spec@fields) row$ofv <- NA_real_
  }

  row
}

#' Format time columns based on spec
#' @noRd
format_time_columns <- function(df, spec) {
  time_cols <- intersect(
    c("estimation_time", "covariance_time", "postprocess_time"),
    names(df)
  )

  if (length(time_cols) == 0 || spec@time_format == "seconds") {
    return(df)
  }

  for (col in time_cols) {
    df[[col]] <- format_time_value(df[[col]], spec@time_format)
  }

  df
}

#' Format time values based on format setting
#' @noRd
format_time_value <- function(seconds, format) {
  if (all(is.na(seconds))) return(seconds)

  switch(
    format,
    "minutes" = seconds / 60,
    "hours" = seconds / 3600,
    "auto" = {
      max_val <- max(seconds, na.rm = TRUE)
      if (max_val >= 3600) {
        seconds / 3600
      } else if (max_val >= 60) {
        seconds / 60
      } else {
        seconds
      }
    },
    seconds
  )
}

# ==============================================================================
# GT table building
# ==============================================================================

#' Extract SummarySpec from a summary data frame
#'
#' Retrieves the `SummarySpec` attached to a data frame (e.g., from
#' `apply_summary_spec()`). Returns NULL if none is found.
#'
#' @param data Data frame carrying a `summary_spec` attribute
#'
#' @return A SummarySpec object or NULL
#' @export
get_summary_spec <- function(data) {
  spec <- attr(data, "summary_spec")
  if (is.null(spec)) {
    return(NULL)
  }
  if (!S7::S7_inherits(spec, SummarySpec)) {
    stop("Attached summary_spec is not a SummarySpec object")
  }
  spec
}

#' Build label map for summary table columns
#' @noRd
build_summary_label_map <- function() {
  list(
    model = "Model",
    based_on = "Reference",
    description = "Description",
    n_parameters = "No. Params",
    problem = "Problem",
    number_data_records = "Records",
    number_subjects = "Subjects",
    number_obs = "Observations",
    estimation_method = "Method",
    estimation_time = "Est. Time",
    covariance_time = "Cov. Time",
    postprocess_time = "Post Time",
    function_evaluations = "Func. Evals",
    significant_digits = "Sig. Digits",
    ofv = "OFV",
    dofv = gt::md("$\\Delta$OFV"),
    condition_number = "Cond. No.",
    termination_status = "Termination",
    pvalue = "p-value",
    df = "df"
  )
}

#' Create summary table from prepared data
#'
#' Creates a formatted gt table from summary data prepared by apply_summary_spec().
#'
#' @param data Data frame from apply_summary_spec()
#'
#' @return A gt table object
#' @export
make_summary_table <- function(data) {
  if (!requireNamespace("dplyr", quietly = TRUE)) {
    stop("Package 'dplyr' is required for make_summary_table()")
  }
  if (!requireNamespace("gt", quietly = TRUE)) {
    stop(
      "Package 'gt' is required for make_summary_table(). ",
      "Install it with 'rv add gt'"
    )
  }

  spec <- attr(data, "summary_spec")
  if (is.null(spec)) {
    stop("SummarySpec not found. Run apply_summary_spec(tree, spec) first.")
  }

  # Pre-merge pvalue with df before creating table
  if (all(c("pvalue", "df") %in% names(data))) {
    use_scientific <- spec@pvalue_scientific
    n_sig <- spec@n_sigfig
    data$pvalue <- vapply(
      seq_len(nrow(data)),
      function(i) {
        pval <- data$pvalue[i]
        df_val <- data$df[i]
        if (is.na(pval) || is.na(df_val)) {
          return(NA_character_)
        }
        if (use_scientific) {
          format_p <- format(pval, scientific = TRUE, digits = n_sig)
          sprintf("%s (df = %d)", format_p, df_val)
        } else {
          sprintf("%s (df = %d)", signif(pval, n_sig), df_val)
        }
      },
      character(1)
    )
    data$df <- NULL
  }

  # Find empty columns to hide
  empty_cols <- if (spec@hide_empty_columns) find_empty_columns(data) else
    character(0)

  # Build labels only for columns that exist
  label_map <- build_summary_label_map()
  label_map <- label_map[intersect(names(label_map), names(data))]

  # Get time format suffix for column labels
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

  table <- gt::gt(data)

  # Hide empty columns
  if (length(empty_cols) > 0) {
    table <- table |> gt::cols_hide(dplyr::all_of(empty_cols))
  }

  # Apply labels and formatting
  table <- table |>
    gt::cols_label(!!!label_map)

  # Format OFV and dOFV with decimals
  ofv_cols <- intersect(c("ofv", "dofv"), names(data))
  if (length(ofv_cols) > 0) {
    table <- table |>
      gt::fmt_number(
        columns = dplyr::all_of(ofv_cols),
        decimals = spec@n_decimals_ofv
      )
  }

  # Format other numeric columns with sigfig
  table <- table |>
    gt::fmt_number(
      columns = dplyr::any_of(c(
        "condition_number",
        "estimation_time",
        "covariance_time",
        "postprocess_time"
      )),
      n_sigfig = spec@n_sigfig
    )

  table <- table |>
    gt::sub_missing(columns = dplyr::everything(), missing_text = "")

  # Add title and styling
  table <- table |>
    gt::tab_header(title = spec@title) |>
    gt::tab_style(
      style = gt::cell_text(weight = "bold"),
      locations = list(
        gt::cells_column_labels(dplyr::everything()),
        gt::cells_title(groups = "title")
      )
    )

  # Build summary stats for conditional footnotes
  summary_stats <- list(
    has_ofv = "ofv" %in% names(data) && any(!is.na(data$ofv)),
    has_dofv = "dofv" %in% names(data) && any(!is.na(data$dofv)),
    has_cond_num = "condition_number" %in%
      names(data) &&
      any(!is.na(data$condition_number)),
    has_pvalue = "pvalue" %in% names(data) && any(!is.na(data$pvalue)),
    dofv_excluded = isTRUE(attr(data, "dofv_excluded_nobs_mismatch"))
  )

  # Add conditional footnotes
  table <- add_conditional_footnotes(
    table,
    data,
    spec,
    summary_stats = summary_stats
  )

  table
}

#' Get time format suffix for column labels
#' @noRd
get_time_suffix <- function(time_format, data) {
  if (time_format == "seconds") {
    return("s")
  } else if (time_format == "minutes") {
    return("min")
  } else if (time_format == "hours") {
    return("h")
  } else if (time_format == "auto") {
    # Determine which format was used based on data values
    time_cols <- intersect(
      c("estimation_time", "covariance_time", "postprocess_time"),
      names(data)
    )
    if (length(time_cols) == 0) return("s")

    max_vals <- vapply(
      time_cols,
      function(col) max(data[[col]], na.rm = TRUE),
      numeric(1)
    )
    max_val <- max(max_vals, na.rm = TRUE)

    if (is.na(max_val) || max_val == 0) {
      return("s")
    } else if (max_val < 1) {
      # Values were divided by 3600 (hours)
      return("h")
    } else if (max_val < 60) {
      # Could be minutes or hours - check original magnitude
      # If max is < 1, it was hours; if 1-60, could be either
      # This is a heuristic - "auto" format already transformed the data
      return("min")
    } else {
      return("s")
    }
  }

  NULL
}
