# =============================================================================
# User-facing DSL functions
# ==============================================================================

#' Create section assignment rules
#'
#' @param ... Formula expressions like `kind == "THETA" ~ "Structural Parameters"`
#' @return List of quosures for use in TableSpec
#' @export
section_rules <- function(...) {
  rlang::enquos(...)
}

#' Create row filter rules
#'
#' @param ... Filter expressions like `!fixed`, `diagonal`
#' @return List of quosures for use in TableSpec
#' @export
filter_rules <- function(...) {
  rlang::enquos(...)
}

# ==============================================================================
# TableSpec S7 Class
# ==============================================================================

#' Table specification for parameter tables
#'
#' @param display_transforms Named list specifying which transforms to apply
#'   for display. Names are parameter kinds (theta, omega, sigma), values are
#'   which columns to transform ("all", "estimate", "cv", "rse", "ci", "symbol").
#' @param sections List of section rules created with `section_rules()`
#' @param row_filter List of filter rules created with `filter_rules()`
#' @param columns Character vector of columns to include in output
#' @param drop_columns Character vector of columns to exclude from output
#' @param ci_level Confidence interval level, between 0 and 1. Default is 0.95
#'   for 95% confidence intervals.
#' @param n_sigfig Number of significant figures for numeric formatting in the
#'   output table. Must be a positive integer. Default is 3.
#' @param name_source Which name field to use from ModelComments: "name" (default),
#'   "display", or "nonmem_name". Controls how parameter names appear in the output
#'   table. Use "nonmem_name" to show raw NONMEM names like "THETA1", "OMEGA(1,1)".
#' @param show_description Logical. If TRUE, adds a description column enriched
#'   from ModelComments. Default is FALSE.
#' @param show_associated_theta Logical. If TRUE (default), omega parameter names
#'   include the associated theta in parentheses (e.g., "OM1 (CL)"). If FALSE,
#'   shows just the omega name without the associated theta suffix.
#'
#' @export
TableSpec <- S7::new_class(
  "TableSpec",
  properties = list(
    display_transforms = S7::new_property(
      class = S7::class_list,
      default = list(theta = "all", omega = "all", sigma = "all")
    ),
    sections = S7::new_property(
      class = S7::class_list,
      default = list()
    ),
    row_filter = S7::new_property(
      class = S7::class_list,
      default = list()
    ),
    columns = S7::new_property(
      class = S7::class_character,
      default = c(
        "name",
        "symbol",
        "unit",
        "estimate",
        "ci_low",
        "ci_high",
        "fixed",
        "cv",
        "corr",
        "sd",
        "rse",
        "shrinkage"
      )
    ),
    drop_columns = S7::new_property(
      class = S7::class_character,
      default = character(0)
    ),
    ci_level = S7::new_property(
      class = S7::class_numeric,
      default = 0.95
    ),
    n_sigfig = S7::new_property(
      class = S7::class_numeric,
      default = 3
    ),
    name_source = S7::new_property(
      class = S7::class_character,
      default = "name"
    ),
    show_description = S7::new_property(
      class = S7::class_logical,
      default = FALSE
    ),
    show_associated_theta = S7::new_property(
      class = S7::class_logical,
      default = TRUE
    )
  ),
  validator = function(self) {
    valid_kinds <- c("theta", "omega", "sigma")
    valid_transform_cols <- c(
      "all",
      "estimate",
      "cv",
      "rse",
      "ci",
      "ci_low",
      "ci_high",
      "symbol"
    )
    valid_table_cols <- c(
      "name",
      "description",
      "symbol",
      "unit",
      "estimate",
      "ci_low",
      "ci_high",
      "fixed",
      "cv",
      "corr",
      "sd",
      "rse",
      "shrinkage"
    )

    dt <- self@display_transforms
    if (!all(names(dt) %in% valid_kinds)) {
      bad <- setdiff(names(dt), valid_kinds)
      return(sprintf(
        "@display_transforms names must be in: %s\n  Got: %s",
        paste(valid_kinds, collapse = ", "),
        paste(bad, collapse = ", ")
      ))
    }

    col_values <- unlist(dt)
    if (length(col_values) > 0 && !all(col_values %in% valid_transform_cols)) {
      bad <- setdiff(col_values, valid_transform_cols)
      return(sprintf(
        "@display_transforms values must be in: %s\n  Got: %s",
        paste(valid_transform_cols, collapse = ", "),
        paste(bad, collapse = ", ")
      ))
    }

    if (!all(vapply(self@sections, rlang::is_formula, logical(1)))) {
      return("@section rules must be created with section_rules()")
    }

    if (
      length(self@row_filter) > 0 &&
        !all(vapply(self@row_filter, rlang::is_quosure, logical(1)))
    ) {
      return("@row_filter rules must be created with filter_rules()")
    }

    if (!all(self@columns %in% valid_table_cols)) {
      bad <- setdiff(self@columns, valid_table_cols)
      return(sprintf(
        "@columns must be in: %s\n  Got: %s",
        paste(valid_table_cols, collapse = ", "),
        paste(bad, collapse = ", ")
      ))
    }

    if (
      length(self@drop_columns) > 0 &&
        !all(self@drop_columns %in% valid_table_cols)
    ) {
      bad <- setdiff(self@drop_columns, valid_table_cols)
      return(sprintf(
        "@drop_columns must be in: %s\n  Got: %s",
        paste(valid_table_cols, collapse = ", "),
        paste(bad, collapse = ", ")
      ))
    }

    if (self@ci_level <= 0 || self@ci_level >= 1) {
      return("@ci_level must be between 0 and 1 (exclusive)")
    }

    if (
      length(self@n_sigfig) != 1 ||
        self@n_sigfig < 1 ||
        self@n_sigfig != floor(self@n_sigfig)
    ) {
      return("@n_sigfig must be a positive whole number")
    }

    if (!self@name_source %in% c("name", "display", "nonmem_name")) {
      return("@name_source must be 'name', 'display', or 'nonmem_name'")
    }

    if (length(self@show_description) != 1 || is.na(self@show_description)) {
      return("@show_description must be TRUE or FALSE")
    }

    if (
      length(self@show_associated_theta) != 1 ||
        is.na(self@show_associated_theta)
    ) {
      return("@show_associated_theta must be TRUE or FALSE")
    }
  },
  constructor = function(
    display_transforms = list(),
    sections = list(),
    row_filter = list(),
    columns = NULL,
    drop_columns = character(0),
    ci_level = 0.95,
    n_sigfig = 3,
    name_source = "name",
    show_description = FALSE,
    show_associated_theta = TRUE
  ) {
    if (!is.list(display_transforms)) {
      stop(
        "@display_transforms must be a list, not a ",
        class(display_transforms)[1]
      )
    }

    if (length(display_transforms) > 0 && !is.null(names(display_transforms))) {
      names(display_transforms) <- tolower(names(display_transforms))
    }

    for (kind in c("theta", "omega", "sigma")) {
      if (!kind %in% names(display_transforms)) {
        display_transforms[[kind]] <- "all"
      }
    }

    if (is.null(columns)) {
      columns <- c(
        "name",
        "symbol",
        "unit",
        "estimate",
        "ci_low",
        "ci_high",
        "fixed",
        "cv",
        "corr",
        "sd",
        "rse",
        "shrinkage"
      )
    }
    S7::new_object(
      S7::S7_object(),
      display_transforms = display_transforms,
      sections = sections,
      row_filter = row_filter,
      columns = columns,
      drop_columns = drop_columns,
      ci_level = ci_level,
      n_sigfig = n_sigfig,
      name_source = name_source,
      show_description = show_description,
      show_associated_theta = show_associated_theta
    )
  }
)

# ==============================================================================
# Apply spec to parameter data
# ==============================================================================

#' Apply table specification to parameter data
#'
#' Enriches parameter data with transforms, CIs, sections, and display names.
#'
#' @param params Data frame from get_parameters()
#' @param info ModelComments object from get_model_parameter_info()
#' @param spec A TableSpec object
#' @importFrom rlang .data
#'
#' @return Enriched data frame ready for table building
#' @export
apply_table_spec <- function(params, info, spec) {
  if (!requireNamespace("dplyr", quietly = TRUE)) {
    stop("Package 'dplyr' is required for apply_table_spec()")
  }
  if (!S7::S7_inherits(spec, TableSpec)) {
    stop("spec must be a TableSpec object")
  }

  dt_kinds <- build_display_transforms(spec)
  col_values <- unlist(spec@display_transforms)

  # Build dt_* column expressions
  dt_exprs <- lapply(names(dt_kinds), function(group) {
    kinds <- dt_kinds[[group]]
    rlang::expr(dplyr::if_else(
      .data$kind %in% !!kinds,
      .data$transforms,
      "identity"
    ))
  }) |>
    stats::setNames(paste0("dt_", names(dt_kinds)))

  # Helper to get the right dt column for a given output column
  dt_for <- function(col) {
    if (col %in% col_values) paste0("dt_", col) else "dt_all"
  }

  df <- params |>
    dplyr::mutate(
      transforms = get_parameter_transform(info, .data$name),
      unit = get_parameter_unit(info, .data$name),
      !!!dt_exprs,
      cv = compute_cv(.data$estimate, .data$kind, .data[[dt_for("cv")]]),
      rse = compute_rse(
        .data$estimate,
        .data$stderr,
        .data$kind,
        .data[[dt_for("rse")]]
      ),
      ci_low = compute_ci(
        .data$estimate,
        .data$stderr,
        spec@ci_level,
        .data[[dt_for("ci")]]
      )$lower,
      ci_high = compute_ci(
        .data$estimate,
        .data$stderr,
        spec@ci_level,
        .data[[dt_for("ci")]]
      )$upper,
      estimate = transform_value(.data$estimate, .data[[dt_for("estimate")]]),
      symbol = param_symbol_md(
        .data$kind,
        .data$random_effect,
        .data[[dt_for("symbol")]]
      ),
      is_summary = FALSE
    )

  # Add description column FIRST (before name transformation)
  # This ensures we match on original/untransformed names
  if (spec@show_description) {
    df <- enrich_description(df, info)
    # Add "description" to columns if not already present
    if (
      !"description" %in% spec@drop_columns &&
        !"description" %in% spec@columns
    ) {
      insert_after <- match("name", spec@columns)
      if (is.na(insert_after)) insert_after <- 1
      spec@columns <- append(spec@columns, "description", after = insert_after)
    }
  }

  # Apply name replacement based on spec@name_source
  df <- apply_name_source(
    df,
    info,
    spec@name_source,
    spec@show_associated_theta
  )

  # Apply section rules AFTER name transformation (consistent with row_filter)
  df <- df |>
    dplyr::mutate(
      section = build_section(dplyr::pick(dplyr::everything()), spec)
    )

  # Apply row filter AFTER name transformation so users can filter on display names
  if (length(spec@row_filter) > 0) {
    for (f in spec@row_filter) {
      df <- df |>
        dplyr::filter(!!f)
    }
  }

  attr(df, "table_spec") <- spec
  df
}

# ==============================================================================
# TableSpec helper functions
# ==============================================================================

#' Build display transform mapping from spec
#' @noRd
build_display_transforms <- function(spec) {
  if (!S7::S7_inherits(spec, TableSpec)) {
    stop("spec must be a TableSpec object")
  }

  dt <- spec@display_transforms
  groups <- unique(unlist(dt))

  dt_kinds <- lapply(groups, function(group) {
    kinds <- names(dt)[vapply(
      dt,
      function(x) {
        !is.null(x) && ("all" %in% x || group %in% x)
      },
      logical(1)
    )]
    toupper(kinds)
  }) |>
    stats::setNames(groups)

  # Always provide dt_all as a fallback transform mapping for every kind
  if (!"all" %in% names(dt_kinds)) {
    dt_kinds[["all"]] <- toupper(names(dt))
  }

  dt_kinds
}

#' Build section assignments using case_when
#' @noRd
build_section <- function(data, spec) {
  if (!S7::S7_inherits(spec, TableSpec)) {
    stop("spec must be a TableSpec object")
  }

  rules <- spec@sections
  if (length(rules) == 0) {
    return(rep(NA_character_, nrow(data)))
  }

  # Convert quosures to case_when format
  # Each quosure wraps a formula like: kind == "THETA" ~ "Structural model parameters"
  args <- lapply(rules, function(q) {
    rlang::eval_tidy(q, data = data)
  })

  dplyr::case_when(!!!args)
}

#' Get section order from spec
#' @noRd
get_section_order <- function(spec) {
  if (!S7::S7_inherits(spec, TableSpec)) {
    stop("spec must be a TableSpec object")
  }

  vapply(
    spec@sections,
    function(rule) {
      rlang::f_rhs(rlang::eval_tidy(rule))
    },
    character(1)
  )
}

#' Apply name source replacement
#'
#' Replaces parameter names based on the name_source setting.
#'
#' @param df Data frame with name and kind columns
#' @param info ModelComments object
#' @param name_source "name", "display", or "nonmem_name"
#' @param show_associated_theta If TRUE, append (theta) suffix to omega names
#' @return Data frame with names replaced
#' @noRd
apply_name_source <- function(
  df,
  info,
  name_source,
  show_associated_theta = TRUE
) {
  # Get the labels data frame (row names are NONMEM names)
  labels <- get_parameter_names(info)

  # Build a lookup table with multiple keys per parameter
  build_lookup_rows <- function(comments, kind_label) {
    lapply(names(comments), function(nonmem) {
      cmt <- comments[[nonmem]]

      # Determine the target display value based on name_source
      if (!nonmem %in% rownames(labels)) {
        target <- nonmem
      } else if (name_source == "nonmem_name") {
        target <- nonmem
      } else if (
        name_source == "display" && !is.na(labels[nonmem, "display"])
      ) {
        target <- labels[nonmem, "display"]
      } else if (!is.na(labels[nonmem, "name"])) {
        target <- labels[nonmem, "name"]
      } else {
        target <- nonmem
      }

      # Add associated_theta suffix for omega if requested
      if (
        show_associated_theta &&
          S7::S7_inherits(cmt, OmegaComment) &&
          !is.null(cmt@associated_theta)
      ) {
        theta_str <- paste(cmt@associated_theta, collapse = "-")
        target <- paste0(target, " (", theta_str, ")")
      }

      # Build keys: nonmem_name, user name, display name
      keys <- c(nonmem)
      if (!is.null(cmt@name)) {
        keys <- c(keys, cmt@name)
      }
      if (!is.null(cmt@display)) {
        keys <- c(keys, cmt@display)
      }

      data.frame(
        key = keys,
        display = target,
        kind = kind_label,
        stringsAsFactors = FALSE
      )
    }) |>
      dplyr::bind_rows()
  }

  lookup <- dplyr::bind_rows(
    build_lookup_rows(info@theta, "THETA"),
    build_lookup_rows(info@omega, "OMEGA"),
    build_lookup_rows(info@sigma, "SIGMA")
  ) |>
    dplyr::distinct(.data$key, .data$kind, .keep_all = TRUE)

  df |>
    dplyr::mutate(
      .match_idx = match(
        paste(.data$name, .data$kind),
        paste(lookup$key, lookup$kind)
      ),
      .display = lookup$display[.data$.match_idx],
      name = dplyr::coalesce(.data$.display, .data$name)
    ) |>
    dplyr::select(-".match_idx", -".display")
}

#' Enrich description column from ModelComments
#'
#' Adds a description column by matching parameter names to ModelComments.
#'
#' @param df Data frame with name and kind columns
#' @param info ModelComments object
#' @return Data frame with description column added
#' @noRd
enrich_description <- function(df, info) {
  # Build lookup table: keys per comment mapped to descriptions
  build_desc_rows <- function(comments, kind_label) {
    lapply(names(comments), function(nonmem) {
      cmt <- comments[[nonmem]]
      desc <- cmt@description
      if (is.null(desc)) desc <- NA_character_

      keys <- c(nonmem)

      if (!is.null(cmt@name)) {
        keys <- c(keys, cmt@name)

        if (
          S7::S7_inherits(cmt, OmegaComment) &&
            !is.null(cmt@associated_theta)
        ) {
          keys <- c(keys, paste0(cmt@name, " (", cmt@associated_theta[1], ")"))
        }
      }

      if (!is.null(cmt@display)) {
        keys <- c(keys, cmt@display)
      }

      data.frame(
        key = keys,
        description = desc,
        kind = kind_label,
        stringsAsFactors = FALSE
      )
    }) |>
      dplyr::bind_rows()
  }

  lookup <- dplyr::bind_rows(
    build_desc_rows(info@theta, "THETA"),
    build_desc_rows(info@omega, "OMEGA"),
    build_desc_rows(info@sigma, "SIGMA")
  ) |>
    dplyr::distinct(.data$key, .data$kind, .keep_all = TRUE)

  df |>
    dplyr::mutate(
      .match_idx = match(
        paste(.data$name, .data$kind),
        paste(lookup$key, lookup$kind)
      ),
      description = lookup$description[.data$.match_idx]
    ) |>
    dplyr::select(-".match_idx")
}

# ==============================================================================
# Footnote helpers
# ==============================================================================

#' Detect which statistics are used in a parameter table
#'
#' @param params Parameter data frame (after apply_table_spec)
#' @return Named list of logicals indicating which stats are present
#' @noRd
detect_table_statistics <- function(params) {
  list(
    # Column presence
    has_ci = all(c("ci_low", "ci_high") %in% names(params)) &&
      any(!is.na(params$ci_low)),
    has_rse = "rse" %in% names(params) && any(!is.na(params$rse)),
    has_shrinkage = "shrinkage" %in%
      names(params) &&
      any(!is.na(params$shrinkage)),

    # Merged column statistics (cv/sd/corr)
    has_cv = "cv" %in% names(params) && any(!is.na(params$cv)),
    has_sd = "sd" %in%
      names(params) &&
      any(!is.na(params$sd) & is.na(params$cv) & is.na(params$corr)),
    has_corr = "corr" %in% names(params) && any(!is.na(params$corr)),

    # Formula-specific (need to know WHICH CV formula)
    has_lognormal_omega_cv = "cv" %in%
      names(params) &&
      "transforms" %in% names(params) &&
      any(
        !is.na(params$cv) &
          params$kind == "OMEGA" &
          tolower(params$transforms) == "lognormal"
      ),
    has_proportional_sigma_cv = "cv" %in%
      names(params) &&
      any(!is.na(params$cv) & params$kind == "SIGMA")
  )
}

#' Add conditional footnotes based on table contents
#'
#' @param table A gt table object
#' @param params Parameter data frame
#' @param spec TableSpec object (for ci_level)
#' @return gt table with appropriate footnotes added
#' @noRd
add_conditional_footnotes <- function(table, params, spec) {
  stats <- detect_table_statistics(params)
  ci_pct <- if (!is.null(spec)) round(spec@ci_level * 100) else 95

  # Build abbreviation list dynamically
  abbrevs <- character(0)
  if (stats$has_ci) abbrevs <- c(abbrevs, "CI = confidence intervals")
  if (stats$has_rse) abbrevs <- c(abbrevs, "RSE = relative standard error")
  if (stats$has_cv) abbrevs <- c(abbrevs, "CV = coefficient of variation")
  if (stats$has_sd) abbrevs <- c(abbrevs, "SD = standard deviation")
  if (stats$has_corr) abbrevs <- c(abbrevs, "Corr = correlation")

  # Add abbreviations footnote if any exist
  if (length(abbrevs) > 0) {
    table <- table |>
      gt::tab_footnote("Abbreviations:") |>
      gt::tab_footnote(paste(abbrevs, collapse = "; "))
  }

  # Add CI formula if CI columns are used
  if (stats$has_ci) {
    table <- table |>
      gt::tab_footnote(
        footnote = gt::md(sprintf(
          "%d%% CI: $\\mathrm{Estimate} \\pm z_{%.3g} \\cdot \\mathrm{SE}$",
          ci_pct,
          (1 - ci_pct / 100) / 2
        ))
      )
  }

  # Add CV formula for log-normal omega if applicable
  if (stats$has_lognormal_omega_cv) {
    table <- table |>
      gt::tab_footnote(
        footnote = gt::md(
          paste0(
            "CV% for log-normal $\\Omega$ diagonals: ",
            "$\\sqrt{\\exp(\\mathrm{Estimate}) - 1} \\times 100$"
          )
        )
      )
  }

  # Add CV formula for proportional error (sigma) if applicable
  if (stats$has_proportional_sigma_cv) {
    table <- table |>
      gt::tab_footnote(
        gt::md(
          "CV% of proportional error: $\\sqrt{\\mathrm{Estimate}} \\times 100$"
        )
      )
  }

  table
}

# ==============================================================================
# Data transformation helpers
# ==============================================================================

#' Add summary rows to parameter table
#'
#' Appends OFV and condition number rows to the parameter table.
#'
#' @param params Enriched parameter data frame from `apply_table_spec()`
#' @param sum Summary object from `get_model_summary()`, or NULL to skip
#'
#' @importFrom rlang .data
#'
#' @return Data frame with summary rows appended
#' @export
add_summary_rows <- function(params, sum) {
  if (!requireNamespace("dplyr", quietly = TRUE)) {
    stop("Package 'dplyr' is required for add_summary_rows()")
  }
  if (is.null(sum)) {
    return(params)
  }

  ofv_val <- dplyr::last(sum$minimization_results$ofv)
  cn_val <- dplyr::last(sum$minimization_results$condition_number)

  sum_df <- data.frame(
    name = c("OFV", "Condition Number"),
    symbol = NA_character_,
    unit = NA_character_,
    estimate = c(ofv_val, cn_val),
    ci_low = NA_real_,
    ci_high = NA_real_,
    fixed = NA,
    cv = NA_real_,
    corr = NA_real_,
    sd = NA_real_,
    rse = NA_real_,
    shrinkage = NA_real_,
    section = "Other",
    is_summary = TRUE
  )

  result <- params |>
    dplyr::mutate(is_summary = FALSE) |>
    dplyr::bind_rows(sum_df)

  spec <- attr(result, "table_spec")
  if (is.null(spec)) {
    stop(
      "TableSpec not found. Run apply_table_spec(params, info, spec) first."
    )
  }
  if (!is.null(spec)) {
    result <- order_sections(result, spec)
    attr(result, "table_spec") <- spec
  }

  result
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
    "is_summary",
    "kind",
    "random_effect",
    "diagonal",
    "transforms"
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
# Formatting helpers (Greek symbols, markdown)
# ==============================================================================

#' Convert parameter kind to Greek symbol in LaTeX math notation
#'
#' Returns raw LaTeX (without $..$ delimiters) for use in param_symbol_md().
#' @noRd
greek_to_latex <- function(kind, random_effect) {
  stopifnot(length(kind) == length(random_effect))

  n <- length(kind)
  out <- rep(NA_character_, n)

  # THETA: enumerate in order of appearance
  is_theta <- !is.na(kind) & kind == "THETA"
  if (any(is_theta)) {
    theta_idx <- seq_len(sum(is_theta))
    out[is_theta] <- sprintf("\\theta_{%d}", theta_idx)
  }

  # Helper: from random_effect -> "row,col" for lower triangle
  # e.g. "ETA1" -> "1,1"; "ETA1:ETA2" -> "2,1"
  make_cov_idx <- function(re) {
    nums_list <- regmatches(re, gregexpr("\\d+", re))
    vapply(
      nums_list,
      function(nums_chr) {
        if (length(nums_chr) == 0L) {
          return("")
        }
        nums <- as.integer(nums_chr)

        if (length(nums) == 1L) {
          sprintf("%d,%d", nums, nums) # ETA1 -> (1,1)
        } else {
          r <- max(nums[1:2]) # ETA1:ETA2 -> (2,1)
          c <- min(nums[1:2])
          sprintf("%d,%d", r, c)
        }
      },
      character(1)
    )
  }

  # OMEGA: ETA... -> Omega
  is_omega <- !is.na(kind) & kind == "OMEGA" & !is.na(random_effect)
  if (any(is_omega)) {
    idx_str <- make_cov_idx(random_effect[is_omega])
    out[is_omega] <- sprintf("\\Omega_{(%s)}", idx_str)
  }

  # SIGMA: EPS... -> Sigma
  is_sigma <- !is.na(kind) & kind == "SIGMA" & !is.na(random_effect)
  if (any(is_sigma)) {
    idx_str <- make_cov_idx(random_effect[is_sigma])
    out[is_sigma] <- sprintf("\\Sigma_{(%s)}", idx_str)
  }

  out
}

#' Build parameter symbols as LaTeX math expressions
#'
#' Wraps in exp() for LogNormal and logistic for Logit transforms.
#' Returns complete LaTeX math expressions wrapped in $..$.
#' @noRd
param_symbol_md <- function(kind, random_effect, transforms) {
  base_sym <- greek_to_latex(kind, random_effect)

  tr <- transforms
  if (is.factor(tr)) tr <- as.character(tr)

  # Build raw LaTeX expression (without $..$ delimiters)
  latex_expr <- dplyr::case_when(
    !is.na(tr) & tolower(tr) == "lognormal" ~ paste0("\\exp(", base_sym, ")"),
    !is.na(tr) & tolower(tr) == "logit" ~
      paste0("1/(1 + \\exp(-", base_sym, "))"),
    TRUE ~ base_sym
  )

  # Wrap in $..$ for inline LaTeX math (only for non-NA values)
  dplyr::if_else(
    !is.na(latex_expr),
    paste0("$", latex_expr, "$"),
    NA_character_
  )
}

# ==============================================================================
# GT table building
# ==============================================================================

# nolint start: object_usage_linter
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

  # Check for table_spec attribute and order sections if present
  spec <- attr(params, "table_spec")
  if (!is.null(spec)) {
    params <- order_sections(params, spec)
  }

  # Get columns to hide (internal + dt_*)
  dt_cols <- grep("^dt_", names(params), value = TRUE)
  hide_cols <- c(
    ".appear_order",
    "is_summary",
    "kind",
    "random_effect",
    "diagonal",
    "transforms",
    dt_cols
  )
  hide_cols <- intersect(hide_cols, names(params))

  # Build labels only for columns that exist
  ci_pct <- if (!is.null(spec)) round(spec@ci_level * 100) else 95
  label_map <- list(
    name = "Parameter",
    description = "",
    symbol = "",
    unit = "",
    estimate = "Estimate",
    ci_low = sprintf("%d%% CI", ci_pct),
    cv = "",
    corr = "",
    sd = "",
    rse = "RSE (%)",
    shrinkage = "Shrinkage (%)"
  )
  label_map <- label_map[intersect(names(label_map), names(params))]

  # This is a hack that autodetects pdf vs html rendering in qmd to set the escaping
  # on % since gt doesn't do it in cols_merge
  escaped_percnt_pattern_val <- if (knitr::is_latex_output())
    "[CV = {1}\\%]" else "[CV = {1}%]"
  table <- params |>
    gt::gt(groupname_col = "section") |>
    gt::cols_hide(dplyr::all_of(hide_cols))

  # CI merge - only if columns exist
  if (all(c("ci_low", "ci_high", "fixed") %in% names(params))) {
    table <- table |>
      gt::cols_merge(
        columns = c("ci_low", "ci_high", "fixed"),
        rows = !.data$fixed & !.data$is_summary,
        pattern = "[{1}, {2}]"
      ) |>
      gt::cols_merge(
        columns = c("ci_low", "ci_high", "fixed"),
        rows = .data$fixed & !.data$is_summary,
        pattern = "Fixed"
      )
  }

  # Random-effect extra info - only if columns exist
  # Note: Use \\% to escape % for LaTeX (gt needs double backslash)
  if (all(c("cv", "corr", "sd", "fixed") %in% names(params))) {
    table <- table |>
      gt::cols_merge(
        columns = c("cv", "corr", "sd", "fixed"),
        rows = !is.na(.data$cv) & !.data$is_summary,
        pattern = escaped_percnt_pattern_val
      ) |>
      gt::cols_merge(
        columns = c("cv", "corr", "sd", "fixed"),
        rows = !is.na(.data$corr) & !.data$is_summary,
        pattern = "[Corr = {2}]"
      ) |>
      gt::cols_merge(
        columns = c("cv", "corr", "sd", "fixed"),
        rows = !is.na(.data$sd) &
          is.na(.data$cv) &
          is.na(.data$corr) &
          !.data$is_summary,
        pattern = "[SD = {3}]"
      ) |>
      gt::cols_merge(
        columns = c("cv", "corr", "sd", "fixed"),
        rows = .data$fixed & !.data$is_summary,
        pattern = "Fixed"
      )
  }

  n_sigfig <- if (!is.null(spec)) spec@n_sigfig else 3
  table <- table |>
    gt::cols_label(!!!label_map) |>
    gt::fmt_markdown() |>
    gt::fmt_number(
      columns = dplyr::any_of(c(
        "estimate",
        "ci_low",
        "ci_high",
        "rse",
        "shrinkage",
        "cv",
        "corr",
        "sd"
      )),
      n_sigfig = n_sigfig
    ) |>
    gt::sub_missing(columns = dplyr::everything(), missing_text = "")

  if (all(c("ci_low", "ci_high") %in% names(params))) {
    table <- table |>
      gt::sub_missing(
        columns = c("ci_low", "ci_high"),
        rows = !.data$is_summary,
        missing_text = "-"
      )
  }

  table <- table |>
    gt::tab_header("Model Parameters")

  # Add conditional footnotes based on what's actually in the table
  table <- add_conditional_footnotes(table, params, spec)

  table <- table |>
    gt::tab_style(
      style = gt::cell_text(weight = "bold"),
      locations = list(
        gt::cells_column_labels(dplyr::everything()),
        gt::cells_title(groups = c("title", "subtitle")),
        gt::cells_row_groups()
      )
    )

  table <- table |>
    gt::opt_css(css = "td, th { white-space: nowrap; }")

  table
}
