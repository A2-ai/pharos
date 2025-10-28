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

#' Generate OMEGA parameter names based on block structure
#'
#' @param structure_info Block structure (e.g., "Diagonal", "Block(2)")
#' @param start_eta Starting ETA index for this block
#' @param block_size Size of the block
#' @param num_params Number of parameters to generate names for
#' @return Character vector of parameter names
#' @keywords internal
#' @noRd
generate_omega_names <- function(
  structure_info,
  start_eta,
  block_size,
  num_params
) {
  param_names <- c()

  if (structure_info == "Diagonal") {
    # Diagonal: each parameter gets (i,i)
    for (i in seq_len(num_params)) {
      param_names <- c(
        param_names,
        paste0("OMEGA(", start_eta + i - 1, ",", start_eta + i - 1, ")")
      )
    }
  } else if (grepl("^Block", structure_info)) {
    # Block(n): lower triangular matrix, filled column-wise
    # Pattern: (j,j), (j+1,j), (j+1,j+1), (j+2,j), (j+2,j+1), (j+2,j+2), ...
    param_count <- 0
    for (col in 0:(block_size - 1)) {
      for (row in col:(block_size - 1)) {
        param_count <- param_count + 1
        if (param_count <= num_params) {
          param_names <- c(
            param_names,
            paste0("OMEGA(", start_eta + row, ",", start_eta + col, ")")
          )
        }
      }
      if (param_count >= num_params) break
    }
  } else {
    # Fallback: assume diagonal
    for (i in seq_len(num_params)) {
      param_names <- c(
        param_names,
        paste0("OMEGA(", start_eta + i - 1, ",", start_eta + i - 1, ")")
      )
    }
  }

  return(param_names)
}

#' Generate SIGMA parameter names based on block structure
#'
#' @param structure_info Block structure (e.g., "Diagonal", "Block(2)")
#' @param start_eps Starting EPS index for this block
#' @param block_size Size of the block
#' @param num_params Number of parameters to generate names for
#' @return Character vector of parameter names
#' @keywords internal
#' @noRd
generate_sigma_names <- function(
  structure_info,
  start_eps,
  block_size,
  num_params
) {
  param_names <- c()

  if (structure_info == "Diagonal") {
    # Diagonal: each parameter gets (i,i)
    for (i in seq_len(num_params)) {
      param_names <- c(
        param_names,
        paste0("SIGMA(", start_eps + i - 1, ",", start_eps + i - 1, ")")
      )
    }
  } else if (grepl("^Block", structure_info)) {
    # Block(n): lower triangular matrix, filled column-wise
    param_count <- 0
    for (col in 0:(block_size - 1)) {
      for (row in col:(block_size - 1)) {
        param_count <- param_count + 1
        if (param_count <= num_params) {
          param_names <- c(
            param_names,
            paste0("SIGMA(", start_eps + row, ",", start_eps + col, ")")
          )
        }
      }
      if (param_count >= num_params) break
    }
  } else {
    # Fallback: assume diagonal
    for (i in seq_len(num_params)) {
      param_names <- c(
        param_names,
        paste0("SIGMA(", start_eps + i - 1, ",", start_eps + i - 1, ")")
      )
    }
  }

  return(param_names)
}

#' Format parameter table consistently across all print methods
#'
#' @param param_data Data frame with parameter information
#' @param digits Number of digits for formatting
#' @return NULL (prints table to console)
#' @keywords internal
#' @noRd
format_parameter_table_unified <- function(param_data, digits = 4) {
  if (nrow(param_data) == 0) {
    return()
  }

  # Use CLI table formatting for consistent appearance
  if (requireNamespace("cli", quietly = TRUE)) {
    # Format numeric columns for better display
    display_df <- param_data
    for (col in names(display_df)) {
      if (is.numeric(display_df[[col]])) {
        display_df[[col]] <- sprintf(
          paste0("%.", digits, "f"),
          display_df[[col]]
        )
      }
    }

    # Calculate column widths for proper alignment
    col_widths <- sapply(seq_len(ncol(display_df)), function(i) {
      col_data_widths <- nchar(as.character(display_df[, i]))
      header_width <- nchar(names(display_df)[i])

      # Handle NA values and ensure we have a minimum width
      max_width <- max(col_data_widths, header_width, na.rm = TRUE)

      # If max_width is still -Inf (all values were NA), use header width as fallback
      if (is.infinite(max_width) || is.na(max_width)) {
        max_width <- header_width
      }

      # Ensure minimum width is at least 3 characters
      max(max_width, 3)
    })

    # Create properly aligned headers - pad first, then style
    headers <- names(display_df)
    header_parts <- sapply(seq_len(length(headers)), function(i) {
      padded_header <- sprintf("%-*s", col_widths[i], headers[i])
      cli::style_bold(padded_header)
    })

    cli::cat_line(paste(header_parts, collapse = "  "))
    cli::cat_line(paste(
      sapply(col_widths, function(w) paste(rep("\u2500", w), collapse = "")),
      collapse = "  "
    ))

    # Print rows with proper alignment and color styling
    for (i in seq_len(nrow(display_df))) {
      row_parts <- sapply(seq_len(ncol(display_df)), function(j) {
        cell_data <- as.character(display_df[i, j])
        col_name <- names(display_df)[j]

        # Apply padding first (using plain text)
        padded_cell <- sprintf("%-*s", col_widths[j], cell_data)

        # Apply color styling after padding
        if (
          col_name == "Parameter" &&
            grepl("^(THETA|OMEGA|SIGMA|ETA|EPS)", cell_data)
        ) {
          # Parameter names in blue
          padded_cell <- cli::col_blue(padded_cell)
        } else if (col_name == "Fixed" && cell_data == "yes") {
          # Fixed parameters in red
          padded_cell <- cli::col_red(padded_cell)
        } else if (
          (col_name == "Initial" || col_name == "Estimate") &&
            grepl("^[0-9]", cell_data)
        ) {
          # Estimates in green
          padded_cell <- cli::col_green(padded_cell)
        } else if (
          (col_name == "Lower" || col_name == "Upper") &&
            !is.na(cell_data) &&
            cell_data != "NA"
        ) {
          # Bounds in yellow
          padded_cell <- cli::col_yellow(padded_cell)
        }

        return(padded_cell)
      })

      cli::cat_line(paste(row_parts, collapse = "  "))
    }
  } else if (requireNamespace("knitr", quietly = TRUE)) {
    # Fallback to knitr::kable with better formatting
    formatted_table <- knitr::kable(
      param_data,
      format = "simple",
      digits = digits
    )
    cat(formatted_table, sep = "\n")
  } else {
    # Final fallback to base print with better formatting
    print(param_data, row.names = FALSE, digits = digits)
  }
  cli::cli_text("")
}
