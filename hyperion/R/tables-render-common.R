# ==============================================================================
# Shared Rendering Policy + Preprocessing for HyperionTable
# ==============================================================================

#' Default render policy for table preprocessing
#'
#' @param table HyperionTable object
#' @return List with default rendering behavior settings
#' @noRd
default_render_policy <- function(table) {
  list(
    ci = list(
      merge = TRUE,
      pattern = "[%s, %s]",
      missing_text = table@ci_missing_text %||% "-",
      missing_rows = table@ci_missing_rows,
      n_sigfig = table@n_sigfig,
      formatter = NULL
    ),
    numeric = list(
      n_sigfig = table@n_sigfig,
      formatter = NULL
    ),
    missing = list(
      text = table@missing_text,
      apply_to = "all" # "all" | "numeric" | "character"
    )
  )
}

#' Merge render policy overrides into defaults
#'
#' @param default_policy Default policy list
#' @param override Optional override list
#' @return Merged policy list
#' @noRd
merge_render_policy <- function(default_policy, override = NULL) {
  if (is.null(override)) {
    return(default_policy)
  }

  utils::modifyList(default_policy, override)
}

#' Prepare HyperionTable data for rendering
#'
#' Applies shared formatting and missing text policies, then returns a
#' renderer-agnostic payload.
#'
#' @param table HyperionTable object
#' @param policy Optional render policy override
#' @return List with prepared data + rendering metadata
#' @noRd
prepare_table_render <- function(table, policy = NULL) {
  if (!S7::S7_inherits(table, HyperionTable)) {
    stop("table must be a HyperionTable object")
  }

  policy <- merge_render_policy(default_render_policy(table), policy)

  data <- table@data

  if (isTRUE(policy$ci$merge)) {
    data <- merge_ci_columns_data(
      data,
      ci_merges = table@ci_merges,
      ci_missing_text = policy$ci$missing_text,
      ci_missing_rows = policy$ci$missing_rows,
      n_sigfig = policy$ci$n_sigfig,
      pattern = policy$ci$pattern,
      formatter = policy$ci$formatter
    )
  }

  ci_cols <- character(0)
  for (merge in table@ci_merges) {
    ci_cols <- c(ci_cols, merge$ci_low, merge$ci_high)
  }
  if (!isTRUE(policy$ci$merge)) {
    ci_cols <- character(0)
  }

  data <- format_numeric_columns_shared(
    data,
    numeric_cols = table@numeric_cols,
    n_sigfig = policy$numeric$n_sigfig,
    formatter = policy$numeric$formatter,
    skip_cols = ci_cols
  )

  data <- apply_missing_text_policy(
    data,
    missing_text = policy$missing$text,
    apply_to = policy$missing$apply_to
  )

  visible_cols <- setdiff(names(data), table@hide_cols)
  data_cols <- visible_cols
  if (!is.null(table@groupname_col) && table@groupname_col %in% names(data)) {
    data_cols <- union(table@groupname_col, data_cols)
  }
  data <- data[, data_cols, drop = FALSE]
  visible_cols <- names(data)

  list(
    data = data,
    visible_cols = visible_cols,
    groupname_col = table@groupname_col,
    col_labels = table@col_labels,
    spanners = table@spanners,
    bold_locations = table@bold_locations,
    borders = table@borders,
    title = table@title,
    footnotes = table@footnotes
  )
}

#' Merge CI columns into a single display column
#'
#' @param data Data frame
#' @param ci_merges List of CI merge specifications
#' @param ci_missing_text Text for missing CI values in specific rows
#' @param ci_missing_rows Logical vector or row indices indicating which rows
#'   should show ci_missing_text when CI is NA. Others show empty string.
#' @param n_sigfig Number of significant figures for formatting
#' @param pattern sprintf pattern for CI display
#' @param formatter Optional formatter function(x, n_sigfig)
#' @return Data frame with CI columns merged
#' @noRd
merge_ci_columns_data <- function(
  data,
  ci_merges,
  ci_missing_text = "-",
  ci_missing_rows = NULL,
  n_sigfig = 3,
  pattern = "[%s, %s]",
  formatter = NULL
) {
  if (length(ci_merges) == 0) {
    return(data)
  }

  is_missing_row <- function(i) {
    if (is.null(ci_missing_rows)) {
      return(FALSE)
    }
    if (is.logical(ci_missing_rows)) {
      return(isTRUE(ci_missing_rows[i]))
    }
    i %in% ci_missing_rows
  }

  format_ci_value <- function(x) {
    if (!is.null(formatter)) {
      return(formatter(x, n_sigfig))
    }
    format_value(x, n_sigfig)
  }

  for (merge in ci_merges) {
    ci_low <- merge$ci_low
    ci_high <- merge$ci_high

    if (!all(c(ci_low, ci_high) %in% names(data))) {
      next
    }

    merged_values <- vapply(
      seq_len(nrow(data)),
      function(i) {
        low <- data[[ci_low]][i]
        high <- data[[ci_high]][i]
        if (is.na(low) || is.na(high)) {
          if (is_missing_row(i)) {
            return(ci_missing_text)
          }
          return("")
        }
        sprintf(pattern, format_ci_value(low), format_ci_value(high))
      },
      character(1)
    )

    data[[ci_low]] <- merged_values
    data[[ci_high]] <- NULL
  }

  data
}

#' Format a single numeric value
#' @noRd
format_value <- function(x, n_sigfig = 3) {
  if (is.na(x)) return(NA_character_)
  if (is.character(x)) return(x)
  formatC(x, digits = n_sigfig, format = "g")
}

#' Format numeric columns for display
#'
#' @param data Data frame
#' @param numeric_cols Character vector of numeric column names
#' @param n_sigfig Number of significant figures
#' @param formatter Optional formatter function(x, n_sigfig)
#' @param skip_cols Columns to skip formatting
#' @return Data frame with formatted columns
#' @noRd
format_numeric_columns_shared <- function(
  data,
  numeric_cols,
  n_sigfig,
  formatter = NULL,
  skip_cols = character(0)
) {
  if (length(numeric_cols) == 0) {
    return(data)
  }

  format_numeric_value <- function(x) {
    if (is.na(x)) return(NA_character_)
    if (!is.null(formatter)) {
      return(formatter(x, n_sigfig))
    }
    formatC(x, digits = n_sigfig, format = "g")
  }

  for (col in numeric_cols) {
    if (!col %in% names(data)) next
    if (col %in% skip_cols) next
    if (!is.numeric(data[[col]])) next

    data[[col]] <- vapply(
      data[[col]],
      format_numeric_value,
      character(1)
    )
  }

  data
}

#' Apply missing text substitution
#'
#' @param data Data frame
#' @param missing_text Text to substitute for NA values
#' @param apply_to "all", "numeric", or "character"
#' @return Data frame with missing text applied
#' @noRd
apply_missing_text_policy <- function(
  data,
  missing_text = "",
  apply_to = "all"
) {
  if (is.null(missing_text)) {
    return(data)
  }

  target_cols <- names(data)
  if (apply_to == "numeric") {
    target_cols <- names(data)[vapply(data, is.numeric, logical(1))]
  } else if (apply_to == "character") {
    target_cols <- names(data)[vapply(data, is.character, logical(1))]
  }

  for (col in target_cols) {
    if (is.factor(data[[col]])) {
      data[[col]] <- as.character(data[[col]])
    }
    data[[col]][is.na(data[[col]])] <- missing_text
  }

  data
}
