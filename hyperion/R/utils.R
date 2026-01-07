#' Null-coalescing operator
#'
#' Returns the right-hand side if the left-hand side is NULL, otherwise returns the left-hand side.
#'
#' @param x Left-hand side value
#' @param y Right-hand side value (default if x is NULL)
#' @return x if x is not NULL, otherwise y
#' @keywords internal
#' @noRd
`%||%` <- function(x, y) {
  if (is.null(x)) y else x
}

#' Format numbers using significant digits with hyperion options
#'
#' @param x Numeric value(s) to format
#' @param digits Number of significant digits (uses option if NULL)
#' @return Formatted numeric value(s)
#' @keywords internal
#' @noRd
format_hyperion_number <- function(x, digits = NULL) {
  if (is.null(digits)) {
    digits <- getOption("hyperion.significant_number_display", 4)
  }
  signif(x, digits)
}

#' Format all numeric columns in a data frame for display
#'
#' Applies format_hyperion_number to all numeric columns in a data frame.
#' This is the single source of truth for number formatting across all print methods.
#'
#' @param data Data frame to format
#' @param digits Number of significant digits (uses global option if NULL)
#' @return Data frame with numeric columns formatted
#' @keywords internal
#' @noRd
format_display_data <- function(data, digits = NULL) {
  if (nrow(data) == 0) {
    return(data)
  }

  # Step 1: Format all numeric and boolean columns
  formatted_data <- data
  for (col in names(formatted_data)) {
    if (is.numeric(formatted_data[[col]])) {
      formatted_data[[col]] <- format_hyperion_number(
        formatted_data[[col]],
        digits
      )
    } else if (is.logical(formatted_data[[col]])) {
      formatted_data[[col]] <- ifelse(
        formatted_data[[col]] %||% FALSE,
        "Yes",
        "No"
      )
    }
  }

  # Step 2: Remove redundant columns (kind is redundant with table title)
  if ("kind" %in% names(formatted_data)) {
    formatted_data <- formatted_data[,
      !names(formatted_data) %in% "kind",
      drop = FALSE
    ]
  }

  # Step 3: Remove completely empty columns (all NA, empty strings, or whitespace)
  empty_cols <- sapply(formatted_data, function(col) {
    all(is.na(col) | trimws(as.character(col)) == "")
  })

  if (any(empty_cols)) {
    formatted_data <- formatted_data[, !empty_cols, drop = FALSE]
  }

  # Step 4: Rename columns to user-friendly display names
  names(formatted_data) <- sapply(names(formatted_data), function(name) {
    switch(
      name,
      "name" = "Parameter",
      "random_effect" = "Random Effect",
      "estimate" = "Estimate",
      "stderr" = "SE",
      "rse" = "RSE (%)",
      "shrinkage" = "Shrinkage (%)",
      "fixed" = "Fixed",
      name # Default: keep original name
    )
  })

  return(formatted_data)
}

#' Build a shared display table model for console/knit renderers
#'
#' Computes column widths and shared cell styling flags.
#'
#' @param formatted_data Data frame already processed by format_display_data()
#' @param title Table title to display
#' @return List with title, data, col_widths, and style_flags
#' @keywords internal
#' @noRd
build_display_table_model <- function(formatted_data, title) {
  if (nrow(formatted_data) == 0) {
    return(list(
      title = title,
      data = formatted_data,
      col_widths = integer(),
      style_flags = list(red = matrix(FALSE, nrow = 0, ncol = 0))
    ))
  }

  display_data <- formatted_data

  # Calculate column widths for proper alignment (console)
  col_widths <- sapply(seq_len(ncol(display_data)), function(i) {
    col_data_widths <- nchar(as.character(display_data[, i]))
    header_width <- nchar(names(display_data)[i])

    # Handle NA values and ensure we have a minimum width
    max_width <- max(col_data_widths, header_width, na.rm = TRUE)

    # If max_width is still -Inf (all values were NA), use header width as fallback
    if (is.infinite(max_width) || is.na(max_width)) {
      max_width <- header_width
    }

    # Ensure minimum width is at least 3 characters
    max(max_width, 3)
  })

  # Shared styling rules (red highlights)
  rse_threshold <- getOption("hyperion.nonmem_summary.rse_threshold")
  shrinkage_threshold <- getOption(
    "hyperion.nonmem_summary.shrinkage_threshold"
  )
  red_flags <- matrix(
    FALSE,
    nrow = nrow(display_data),
    ncol = ncol(display_data)
  )

  for (i in seq_len(nrow(display_data))) {
    for (j in seq_len(ncol(display_data))) {
      cell_data <- as.character(display_data[i, j])
      col_name <- names(display_data)[j]

      if (col_name == "Correlation") {
        red_flags[i, j] <- TRUE
      } else if (col_name == "Fixed" && cell_data == "Yes") {
        red_flags[i, j] <- TRUE
      } else if (
        col_name == "RSE (%)" &&
          !is.na(suppressWarnings(as.numeric(cell_data))) &&
          suppressWarnings(as.numeric(cell_data)) > rse_threshold
      ) {
        red_flags[i, j] <- TRUE
      } else if (
        col_name == "Shrinkage (%)" &&
          !is.na(suppressWarnings(as.numeric(cell_data))) &&
          suppressWarnings(as.numeric(cell_data)) > shrinkage_threshold
      ) {
        red_flags[i, j] <- TRUE
      }
    }
  }

  list(
    title = title,
    data = display_data,
    col_widths = col_widths,
    style_flags = list(red = red_flags)
  )
}

#' Print data table to console using cli
#'
#' Handles console presentation for any pre-formatted data frame.
#'
#' @param formatted_data Data frame with all numbers pre-formatted as characters
#' @param title Table title to display
#' @return NULL (prints to console)
#' @keywords internal
#' @noRd
print_data_table_console <- function(formatted_data, title) {
  if (nrow(formatted_data) == 0) {
    return()
  }

  model <- build_display_table_model(formatted_data, title)
  display_data <- model$data
  col_widths <- model$col_widths
  red_flags <- model$style_flags$red

  cli::cat_line(" ")
  if (!is.null(model$title)) {
    cli::cli_h2(model$title)
  }

  # Create properly aligned headers - pad first, then style
  headers <- names(display_data)
  header_parts <- sapply(seq_len(length(headers)), function(i) {
    padded_header <- sprintf("%-*s", col_widths[i], headers[i])
    cli::style_bold(padded_header)
  })

  cli::cat_line(" ")
  cli::cat_line(paste(header_parts, collapse = "  "))
  cli::cat_line(paste(
    sapply(col_widths, function(w) paste(rep("\u2500", w), collapse = "")),
    collapse = "  "
  ))

  # Print rows with proper alignment and color styling
  for (i in seq_len(nrow(display_data))) {
    row_parts <- sapply(seq_len(ncol(display_data)), function(j) {
      cell_data <- as.character(display_data[i, j])

      # Apply padding first (using plain text)
      padded_cell <- sprintf("%-*s", col_widths[j], cell_data)

      if (red_flags[i, j]) {
        padded_cell <- cli::col_red(padded_cell)
      }

      return(padded_cell)
    })

    cli::cat_line(paste(row_parts, collapse = "  "))
  }
}

#' Print data table for knit output using kable
#'
#' Handles knit/markdown presentation for any pre-formatted data frame.
#'
#' @param formatted_data Data frame
#' @param title Table title to display
#' @return Character vector of HTML table output
#' @keywords internal
#' @noRd
print_data_table_knit <- function(formatted_data, title) {
  if (nrow(formatted_data) == 0) {
    return(character())
  }

  output <- character()

  # Add title as markdown header
  if (!is.null(title)) {
    output <- c(output, paste0("## ", title), "")
  }

  model <- build_display_table_model(formatted_data, title)
  display_data <- model$data
  red_flags <- model$style_flags$red

  # Apply HTML styling for coloring (same logic as console output)
  for (i in seq_len(nrow(display_data))) {
    for (j in seq_len(ncol(display_data))) {
      if (red_flags[i, j]) {
        cell_data <- as.character(display_data[i, j])
        display_data[i, j] <- paste0(
          '<span style="color: #DD0000;">',
          cell_data,
          '</span>'
        )
      }
    }
  }

  # Determine alignment: numeric columns right, text columns left
  alignment <- sapply(names(display_data), function(col_name) {
    if (is.numeric(display_data[[col_name]])) "r" else "l"
  })

  # Create kable output with NO digits parameter - data is pre-formatted
  table_output <- knitr::kable(
    display_data,
    format = "html",
    align = alignment,
    table.attr = 'class="table table-striped"',
    row.names = FALSE,
    escape = FALSE
  )
  output <- c(output, as.character(table_output), "")

  return(output)
}

#' Generates a tidyverse-esque onAttach message for hyperion options
#'
#' @return a message to display on attach
#' @keywords internal
#' @noRd
#'
#' @examples \dontrun{
#' hyperion_options_message()
#' }
hyperion_options_message <- function() {
  # List of general hyperion options to check
  hyperion_general_options <- c(
    "hyperion.significant_number_display"
  )

  # List of hyperion nonmem object options to check
  hyperion_nonmem_options <- c(
    "hyperion.nonmem_model.show_included_columns",
    "hyperion.nonmem_summary.rse_threshold",
    "hyperion.nonmem_summary.shrinkage_threshold"
  )

  # Process general options
  set_general_options <- c()
  unset_general_options <- c()

  for (opt_name in hyperion_general_options) {
    opt_value <- getOption(opt_name)
    if (!is.null(opt_value)) {
      set_general_options <- c(
        set_general_options,
        paste(opt_name, ":", opt_value)
      )
    } else {
      unset_general_options <- c(
        unset_general_options,
        paste0("options('", opt_name, "') is not set.")
      )
    }
  }

  # Process nonmem object options
  set_nonmem_options <- c()
  unset_nonmem_options <- c()

  for (opt_name in hyperion_nonmem_options) {
    opt_value <- getOption(opt_name)
    if (!is.null(opt_value)) {
      set_nonmem_options <- c(
        set_nonmem_options,
        paste(opt_name, ":", opt_value)
      )
    } else {
      unset_nonmem_options <- c(
        unset_nonmem_options,
        paste0("options('", opt_name, "') is not set.")
      )
    }
  }

  # Check pharos config file status
  pharos_config_status <- find_pharos_config_file()

  # Format .onAttach message
  msg <- "\n\n"

  # Add pharos config section first
  msg <- paste0(
    msg,
    cli::rule(
      left = cli::style_bold("pharos configuration")
    ),
    "\n"
  )

  if (grepl("No pharos.toml config file found", pharos_config_status)) {
    msg <- paste0(
      msg,
      cli::col_red(cli::symbol$cross),
      " ",
      cli::col_red(pharos_config_status),
      "\n"
    )
  } else {
    msg <- paste0(
      msg,
      cli::col_green(cli::symbol$tick),
      " ",
      "pharos.toml found: ",
      pharos_config_status,
      "\n"
    )
  }

  # Add general options section
  if (length(set_general_options)) {
    msg <- paste0(
      msg,
      cli::rule(
        left = cli::style_bold("hyperion options")
      ),
      "\n",
      paste0(
        cli::col_green(cli::symbol$tick),
        " ",
        set_general_options,
        collapse = "\n"
      ),
      "\n"
    )
  }

  # Add nonmem object options section
  if (length(set_nonmem_options)) {
    msg <- paste0(
      msg,
      cli::rule(
        left = cli::style_bold("hyperion nonmem object options")
      ),
      "\n",
      paste0(
        cli::col_green(cli::symbol$tick),
        " ",
        set_nonmem_options,
        collapse = "\n"
      ),
      "\n"
    )
  }

  # Add unset options section (combining both types)
  all_unset_options <- c(unset_general_options, unset_nonmem_options)
  if (length(all_unset_options)) {
    msg <- paste0(
      msg,
      cli::rule(
        left = cli::style_bold("Unset hyperion options")
      ),
      "\n",
      paste0(
        cli::col_red(cli::symbol$cross),
        " ",
        all_unset_options,
        collapse = "\n"
      ),
      "\n"
    )
  }

  paste0(msg, "\n")
}
