# =============================================================================
# User-facing DSL functions
# ==============================================================================

#' Create section assignment rules
#'
#' Creates rules for assigning parameters to named sections in the output table.
#' Rules are evaluated after name transformation, so you can match on the final
#' display name or use the preserved `nonmem_name` and `user_name` columns.
#'
#' @param ... Formula expressions like `kind == "THETA" ~ "Structural Parameters"`
#'
#' @section Available columns:
#' The following columns are available for use in section rules:
#' \itemize{
#'   \item `nonmem_name` - NONMEM identifier ("THETA1", "OMEGA(1,1)")
#'   \item `user_name` - User name from control file comments ("CL", "OM1")
#'   \item `name` - Display name (depends on `name_source` setting)
#'   \item `kind` - Parameter type: "THETA", "OMEGA", or "SIGMA"
#'   \item `diagonal` - TRUE for diagonal matrix elements (variance), FALSE for off-diagonal (covariance)
#'   \item `fixed` - TRUE if parameter is fixed
#' }
#'
#' @return List of quosures for use in TableSpec
#' @examples
#' section_rules(
#'   grepl("~", user_name) ~ "Covariate Effects",
#'   kind == "THETA" ~ "Structural Parameters",
#'   kind == "OMEGA" & diagonal ~ "Between-Subject Variability",
#'   kind == "SIGMA" ~ "Residual Variability"
#' )
#' @export
section_rules <- function(...) {
  rlang::enquos(...)
}

#' Create row filter rules
#'
#' Creates rules for filtering which parameters appear in the output table.
#' Rules are evaluated after name transformation.
#'
#' @param ... Filter expressions like `!fixed`, `diagonal`
#'
#' @section Available columns:
#' The following columns are available for use in filter rules:
#' \itemize{
#'   \item `nonmem_name` - NONMEM identifier ("THETA1", "OMEGA(1,1)")
#'   \item `user_name` - User name from control file comments ("CL", "OM1")
#'   \item `name` - Display name (depends on `name_source` setting)
#'   \item `kind` - Parameter type: "THETA", "OMEGA", or "SIGMA"
#'   \item `diagonal` - TRUE for diagonal matrix elements (variance), FALSE for off-diagonal (covariance)
#'   \item `fixed` - TRUE if parameter is fixed
#' }
#'
#' @return List of quosures for use in TableSpec
#' @examples
#' filter_rules(
#'   !fixed,
#'   diagonal,
#'   kind != "SIGMA"
#' )
#' @export
filter_rules <- function(...) {
  rlang::enquos(...)
}

# ==============================================================================
# TableSpec S7 Class
# ==============================================================================

#' @noRd
expand_ci_alias <- function(cols) {
  if (is.null(cols) || length(cols) == 0) {
    return(cols)
  }
  if ("ci" %in% cols) {
    replace_idx <- which(cols == "ci")
    cols <- unlist(
      lapply(seq_along(cols), function(i) {
        if (i %in% replace_idx) c("ci_low", "ci_high") else cols[[i]]
      }),
      use.names = FALSE
    )
    cols <- cols[!duplicated(cols)]
  }
  cols
}

#' @noRd
valid_table_columns <- function() {
  c(
    "kind",
    "name",
    "random_effect",
    "description",
    "symbol",
    "unit",
    "estimate",
    "stderr",
    "diagonal",
    "ci_low",
    "ci_high",
    "fixed",
    "variability",
    "cv",
    "corr",
    "sd",
    "rse",
    "shrinkage"
  )
}

#' @noRd
comparison_suffix_columns <- function() {
  c(
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
}

#' Table specification for parameter tables
#'
#' @param title Character. Title for the parameter table header. Default is
#'   "Model Parameters".
#' @param name_source Which name field to use from ModelComments: "name" (default),
#'   "display", or "nonmem_name". Controls how parameter names appear in the output
#'   table. Use "nonmem_name" to show raw NONMEM names like "THETA1", "OMEGA(1,1)".
#' @param columns Character vector of columns to include in output.
#' @param add_columns Character vector of columns to append to the column list.
#'   Useful for comparisons when you want to add columns like "pct_change"
#'   without overriding `columns`.
#' @param drop_columns Character vector of columns to exclude from output, or
#'   NULL (default) to include all columns.
#' @param hide_empty_columns Logical. If TRUE, columns that are all NA/empty
#'   are automatically hidden unless explicitly requested via `columns` or
#'   `add_columns`. Default is TRUE.
#' @param sections Section rules created with `section_rules()`.
#' @param row_filter Filter rules created with `filter_rules()`.
#' @param display_transforms Named list specifying which transforms to apply
#'   for display. Names are parameter kinds (theta, omega, sigma), values are
#'   which columns to transform ("all", "estimate", "cv", "rse", "ci", "symbol").
#' @param n_sigfig Number of significant figures for numeric formatting in the
#'   output table. Must be a positive integer. Default is 3.
#' @param ci_level Confidence interval level, between 0 and 1. Default is 0.95
#'   for 95% confidence intervals.
#' @param n_decimals_ofv Number of decimal places for OFV values in summary
#'   footnotes. Use NA to keep significant-figure formatting. Default is NA.
#' @param pvalue_scientific Logical. If TRUE, p-values are formatted, default FALSE
#'   in scientific notation. If FALSE, uses significant figures from n_sigfig.
#'
#' @export
TableSpec <- S7::new_class(
  "TableSpec",
  properties = list(
    title = S7::new_property(
      class = S7::class_character,
      default = "Model Parameters"
    ),
    name_source = S7::new_property(
      class = S7::class_character,
      default = "name"
    ),
    columns = S7::new_property(
      class = S7::class_character,
      default = c(
        "name",
        "symbol",
        "unit",
        "estimate",
        "variability",
        "ci_low",
        "ci_high",
        "rse",
        "shrinkage"
      ),
      setter = function(self, value) {
        S7::prop(self, "columns") <- value
        # if @columns <- is set or mutated we
        # set .columns_provided to true for
        # hide_empty_columns
        self@.columns_provided <- TRUE
        self
      }
    ),
    add_columns = S7::new_property(
      class = S7::class_character | NULL,
      default = NULL,
      setter = function(self, value) {
        S7::prop(self, "add_columns") <- value
        self
      }
    ),
    drop_columns = S7::new_property(
      class = S7::class_character | NULL,
      default = NULL
    ),
    hide_empty_columns = S7::new_property(
      class = S7::class_logical,
      default = TRUE
    ),
    sections = S7::new_property(
      class = S7::class_list,
      default = list()
    ),
    row_filter = S7::new_property(
      class = S7::class_list,
      default = list()
    ),
    display_transforms = S7::new_property(
      class = S7::class_list,
      default = list()
    ),
    n_sigfig = S7::new_property(
      class = S7::class_numeric,
      default = 3
    ),
    ci_level = S7::new_property(
      class = S7::class_numeric,
      default = 0.95
    ),
    n_decimals_ofv = S7::new_property(
      class = S7::class_numeric,
      default = NA_real_
    ),
    pvalue_scientific = S7::new_property(
      class = S7::class_logical,
      default = FALSE
    ),
    .columns_provided = S7::new_property(
      # Internal: TRUE when user explicitly supplies columns.
      class = S7::class_logical,
      default = FALSE
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
    valid_table_cols <- valid_table_columns()

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

    valid_columns <- c(valid_table_cols, "ci", "pct_change")
    if (!all(self@columns %in% valid_columns)) {
      bad <- setdiff(self@columns, valid_columns)
      return(sprintf(
        "@columns must be in: %s\n  Got: %s",
        paste(valid_columns, collapse = ", "),
        paste(bad, collapse = ", ")
      ))
    }

    if (!is.null(self@add_columns)) {
      if (!is.character(self@add_columns)) {
        return(sprintf(
          "@add_columns must be NULL or a character vector. Got: %s",
          class(self@add_columns)[1]
        ))
      }
      if (!all(self@add_columns %in% valid_columns)) {
        bad <- setdiff(self@add_columns, valid_columns)
        return(sprintf(
          "@add_columns must be in: %s\n  Got: %s",
          paste(valid_columns, collapse = ", "),
          paste(bad, collapse = ", ")
        ))
      }
    }

    comparison_cols <- comparison_suffix_columns()
    comparison_drop_cols <- c(
      paste0(comparison_cols, "_1"),
      paste0(comparison_cols, "_2"),
      paste0(comparison_cols, "_left"),
      paste0(comparison_cols, "_right")
    )
    ci_aliases <- c("ci", "ci_1", "ci_2", "ci_left", "ci_right")
    valid_drop_cols <- c(
      valid_table_cols,
      comparison_drop_cols,
      "pct_change",
      ci_aliases
    )
    main_drop_cols <- c(valid_table_cols, "pct_change", "ci")

    comparison_pattern <- paste0(
      "^(",
      paste(comparison_cols, collapse = "|"),
      ")_\\d+$"
    )
    ci_num_pattern <- "^ci_\\d+$"
    pct_change_pattern <- "^pct_change_\\d+$"

    is_valid_drop <- function(col) {
      col %in%
        valid_drop_cols ||
        grepl(comparison_pattern, col) ||
        grepl(ci_num_pattern, col) ||
        grepl(pct_change_pattern, col)
    }

    if (
      length(self@drop_columns) > 0 &&
        !all(vapply(self@drop_columns, is_valid_drop, logical(1)))
    ) {
      bad <- self@drop_columns[
        !vapply(
          self@drop_columns,
          is_valid_drop,
          logical(1)
        )
      ]
      return(sprintf(
        paste(
          "@drop_columns must be in: %s",
          "For comparisons, use numeric suffixes (_1, _2, _3, ...) or _left/_right for two-model tables.",
          "Got: %s",
          sep = "\n"
        ),
        paste(main_drop_cols, collapse = ", "),
        paste(bad, collapse = ", ")
      ))
    }

    if (self@ci_level <= 0 || self@ci_level >= 1) {
      return(sprintf(
        "@ci_level must be between 0 and 1 (exclusive). Got: %s",
        self@ci_level
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

    if (!is.na(self@n_decimals_ofv)) {
      if (
        length(self@n_decimals_ofv) != 1 ||
          self@n_decimals_ofv < 0 ||
          self@n_decimals_ofv != floor(self@n_decimals_ofv)
      ) {
        return(sprintf(
          "@n_decimals_ofv must be NA or a non-negative whole number. Got: %s",
          self@n_decimals_ofv
        ))
      }
    }

    if (!self@name_source %in% c("name", "display", "nonmem_name")) {
      return(sprintf(
        "@name_source must be 'name', 'display', or 'nonmem_name'. Got: '%s'",
        self@name_source
      ))
    }

    if (
      length(self@hide_empty_columns) != 1 || is.na(self@hide_empty_columns)
    ) {
      return(sprintf(
        "@hide_empty_columns must be TRUE or FALSE. Got: %s",
        self@hide_empty_columns
      ))
    }

    if (
      length(self@.columns_provided) != 1 ||
        is.na(self@.columns_provided)
    ) {
      return(sprintf(
        "@.columns_provided must be TRUE or FALSE. Got: %s",
        self@.columns_provided
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
    title = "Model Parameters",
    name_source = "name",
    columns = NULL,
    add_columns = NULL,
    drop_columns = NULL,
    hide_empty_columns = TRUE,
    sections = section_rules(),
    row_filter = filter_rules(),
    display_transforms = list(),
    n_sigfig = 3,
    ci_level = 0.95,
    n_decimals_ofv = NA_real_,
    pvalue_scientific = TRUE
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

    columns_provided <- !is.null(columns)
    if (is.null(columns)) {
      columns <- c(
        "name",
        "symbol",
        "unit",
        "estimate",
        "variability",
        "ci_low",
        "ci_high",
        "rse",
        "shrinkage"
      )
    }
    columns <- expand_ci_alias(columns)
    add_columns <- expand_ci_alias(add_columns)
    spec <- S7::new_object(
      S7::S7_object(),
      display_transforms = display_transforms,
      sections = sections,
      row_filter = row_filter,
      columns = columns,
      drop_columns = drop_columns,
      ci_level = ci_level,
      n_sigfig = n_sigfig,
      add_columns = add_columns,
      n_decimals_ofv = n_decimals_ofv,
      name_source = name_source,
      title = title,
      hide_empty_columns = hide_empty_columns,
      .columns_provided = columns_provided,
      pvalue_scientific = pvalue_scientific
    )
    # setter is called for columns which flips columns provided.
    # this reverts it back to what ever it was.
    spec@.columns_provided <- columns_provided
    spec
  }
)
