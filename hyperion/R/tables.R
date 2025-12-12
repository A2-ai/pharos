# ==============================================================================
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
  },
  constructor = function(
    display_transforms = list(),
    sections = list(),
    row_filter = list(),
    columns = NULL,
    drop_columns = character(0),
    ci_level = 0.95,
    n_sigfig = 3
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
      n_sigfig = n_sigfig
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
      section = build_section(dplyr::pick(dplyr::everything()), spec),
      is_summary = FALSE
    )

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
  if (!is.null(spec)) {
    result <- order_sections(result, spec)
    attr(result, "table_spec") <- spec
  }

  result
}

#' Replace parameter names with display names from ModelComments
#'
#' Safely maps the `name` column from `get_parameters()` output to the display
#' names defined in a `ModelComments` object. Matching is done against NONMEM
#' names and user-defined names, restricted by parameter kind to avoid collisions.
#' Unmatched rows keep their original name.
#'
#' @param params Data frame from `get_parameters()`
#' @param info ModelComments object from `get_model_parameter_info()`
#' @param column Column to replace; default is `"name"`
#'
#' @return Data frame with names replaced by display labels
#' @export
add_display_names <- function(params, info, column = "name") {
  if (!requireNamespace("dplyr", quietly = TRUE)) {
    stop("Package 'dplyr' is required for add_display_names()")
  }
  if (!S7::S7_inherits(info, ModelComments)) {
    stop("info must be a ModelComments object")
  }
  if (!column %in% names(params)) {
    stop("Column '", column, "' not found in params")
  }
  if (!"kind" %in% names(params)) {
    stop("params must contain a 'kind' column to match display names")
  }

  display_map <- get_parameter_display_names(info)

  # Build a lookup table of possible keys per parameter
  build_rows <- function(comments, kind_label) {
    lapply(names(comments), function(nonmem) {
      cmt <- comments[[nonmem]]
      keys <- c(nonmem)

      if (!is.null(cmt@name)) {
        keys <- c(keys, cmt@name)

        # Omega with associated theta gets an additional key "name (theta)"
        if (
          S7::S7_inherits(cmt, Type1OmegaComment) &&
            !is.null(cmt@associated_theta)
        ) {
          keys <- c(keys, paste0(cmt@name, " (", cmt@associated_theta[1], ")"))
        }
      }

      data.frame(
        key = keys,
        display = display_map[[nonmem]],
        kind = kind_label,
        stringsAsFactors = FALSE
      )
    }) |>
      dplyr::bind_rows()
  }

  lookup <- dplyr::bind_rows(
    build_rows(info@theta, "THETA"),
    build_rows(info@omega, "OMEGA"),
    build_rows(info@sigma, "SIGMA")
  )

  # Remove duplicate keys, keeping the first (NONMEM names appear first)
  lookup <- lookup |>
    dplyr::distinct(.data$key, .data$kind, .keep_all = TRUE)

  params |>
    dplyr::mutate(
      .match_idx = match(
        paste(.data[[column]], .data$kind),
        paste(lookup$key, lookup$kind)
      ),
      .display = lookup$display[.data$.match_idx],
      !!column := dplyr::coalesce(.data$.display, .data[[column]])
    ) |>
    dplyr::select(-.match_idx, -.display)
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

#' Convert parameter kind to Greek symbol in markdown/HTML
#' @noRd
greek_to_md <- function(kind, random_effect) {
  stopifnot(length(kind) == length(random_effect))

  n <- length(kind)
  out <- rep(NA_character_, n)

  # THETA: enumerate in order of appearance
  is_theta <- !is.na(kind) & kind == "THETA"
  if (any(is_theta)) {
    theta_idx <- seq_len(sum(is_theta))
    out[is_theta] <- sprintf("&theta;<sub>%d</sub>", theta_idx)
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
    out[is_omega] <- sprintf("&Omega;<sub>(%s)</sub>", idx_str)
  }

  # SIGMA: EPS... -> Sigma
  is_sigma <- !is.na(kind) & kind == "SIGMA" & !is.na(random_effect)
  if (any(is_sigma)) {
    idx_str <- make_cov_idx(random_effect[is_sigma])
    out[is_sigma] <- sprintf("&Sigma;<sub>(%s)</sub>", idx_str)
  }

  out
}

#' Build parameter symbols, wrapping in exp() for LogNormal transforms
#' @noRd
param_symbol_md <- function(kind, random_effect, transforms) {
  base_sym <- greek_to_md(kind, random_effect)

  tr <- transforms
  if (is.factor(tr)) tr <- as.character(tr)

  dplyr::if_else(
    !is.na(tr) & tolower(tr) == "lognormal",
    paste0("exp(", base_sym, ")"),
    base_sym
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
  if (all(c("cv", "corr", "sd", "fixed") %in% names(params))) {
    table <- table |>
      gt::cols_merge(
        columns = c("cv", "corr", "sd", "fixed"),
        rows = !is.na(.data$cv) & !.data$is_summary,
        pattern = "[CV = {1}%]"
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
    gt::tab_header("Model Parameters") |>
    gt::tab_footnote("Abbreviations:") |>
    gt::tab_footnote(
      "CI = confidence intervals; RSE = relative standard error; CV = coefficient of variation; SD = standard deviation"
    ) |>
    gt::tab_footnote(
      footnote = gt::md(sprintf(
        "%d%% CI: $\\text{Estimate} \\pm z_{%.3g}\\,\\mathrm{SE}$",
        ci_pct,
        (1 - ci_pct / 100) / 2
      ))
    ) |>
    gt::tab_footnote(
      footnote = gt::md(
        "CV% for log-normal OMEGA diagonals: $\\text{CV\\%} = \\sqrt{e^{\\text{Estimate}} - 1} \\times 100$"
      )
    ) |>
    gt::tab_footnote(
      gt::md(
        "CV% of proportional error: $\\text{CV\\%} = \\sqrt{\\text{Estimate}} \\times 100$"
      )
    ) |>
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
# nolint end
