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
#' @param title Character. Title for the parameter table header. Default is
#'   "Model Parameters".
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
        "variability",
        "ci_low",
        "ci_high",
        "fixed",
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
    ),
    title = S7::new_property(
      class = S7::class_character,
      default = "Model Parameters"
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
      "variability",
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
    show_associated_theta = TRUE,
    title = "Model Parameters"
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
        "variability",
        "ci_low",
        "ci_high",
        "fixed",
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
      show_associated_theta = show_associated_theta,
      title = title
    )
  }
)
