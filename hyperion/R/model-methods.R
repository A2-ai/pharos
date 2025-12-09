#' Print method for hyperion_nonmem_model objects
#'
#' @param x A hyperion_nonmem_model object
#' @param digits Number of significant digits (uses global option if NULL)
#' @param ... Additional arguments (ignored)
#' @return Invisible copy of x
#' @export
print.hyperion_nonmem_model <- function(x, digits = NULL, ...) {
  print_model_header(x)
  print_model_data_info(x)

  # Get all parameter names once from pharos
  all_param_names <- get_model_parameter_names(x)

  # Extract names by parameter type
  theta_names <- names(all_param_names)[grepl("^THETA", names(all_param_names))]
  omega_names <- names(all_param_names)[grepl("^OMEGA", names(all_param_names))]
  sigma_names <- names(all_param_names)[grepl("^SIGMA", names(all_param_names))]

  # Pass pre-computed names to print functions
  print_theta_parameters(x, digits, theta_names)
  print_omega_parameters(x, digits, omega_names)
  print_sigma_parameters(x, digits, sigma_names)

  invisible(x)
}

#' Print model header information
#'
#' @param x A hyperion_nonmem_model object
#' @return NULL (prints to console)
#' @keywords internal
#' @noRd
print_model_header <- function(x) {
  # Header with filename if available
  if (!is.null(x$filename)) {
    cli::cli_h1("NONMEM Model: {x$filename}")
  } else {
    cli::cli_h1("NONMEM Model")
  }

  # Problem information - handle different possible structures
  if (!is.null(x$problem)) {
    if (is.character(x$problem) && length(x$problem) > 0) {
      cli::cli_text("{.strong Problem:} {x$problem}")
    } else if (is.list(x$problem) && !is.null(x$problem$title)) {
      cli::cli_text("{.strong Problem:} {x$problem$title}")
    }
  }

  # Record information
  if (!is.null(x$records)) {
    cli::cli_text("{.strong Records:} {length(x$records)} record blocks")

    # Count record types
    if (length(x$records) > 0) {
      record_types <- sapply(x$records, function(r) {
        if (is.list(r) && !is.null(r$record_type)) {
          r$record_type
        } else {
          "Unknown"
        }
      })
      record_counts <- table(record_types)

      cli::cli_text("{.strong Record Types:}")
      for (i in seq_along(record_counts)) {
        type <- names(record_counts)[i]
        count <- record_counts[i]
        cli::cli_text("  \u2022 {type}: {count}")
      }
    }
  }
}

#' Print model data and input column information
#'
#' @param x A hyperion_nonmem_model object
#' @return NULL (prints to console)
#' @keywords internal
#' @noRd
print_model_data_info <- function(x) {
  # Dataset information
  if (!is.null(x$data)) {
    if (is.character(x$data) && length(x$data) > 0) {
      cli::cli_text("{.strong Dataset:} {x$data}")
    } else if (is.list(x$data)) {
      if (!is.null(x$data$path)) {
        cli::cli_text("{.strong Dataset:} {x$data$path}")
      }

      # Show ignore conditions if any
      if (!is.null(x$data$ignore) && length(x$data$ignore) > 0) {
        ignore_markers <- sapply(x$data$ignore, format_ignore_condition)
        cli::cli_text(
          "{.strong Ignore:} {paste(ignore_markers, collapse = ', ')}"
        )
      }

      # Show number of records if available
      if (!is.null(x$data$num_records)) {
        cli::cli_text("{.strong Records:} {x$data$num_records}")
      }
    }
  }

  # Input columns information
  if (!is.null(x$input_columns) && length(x$input_columns) > 0) {
    # Handle different column types (Included, Dropped, Aliased)
    included_cols <- c()
    dropped_cols <- c()
    aliased_cols <- c()

    for (col in x$input_columns) {
      if (!is.null(col$Included)) {
        included_cols <- c(included_cols, col$Included)
      } else if (!is.null(col$Dropped)) {
        dropped_cols <- c(dropped_cols, col$Dropped)
      } else if (!is.null(col$Aliased)) {
        aliased_cols <- c(
          aliased_cols,
          paste0(col$Aliased$from, "\u2192", col$Aliased$to)
        )
      }
    }

    if (
      length(included_cols) > 0 &&
        getOption("hyperion.nonmem_model.show_included_columns", FALSE)
    ) {
      cli::cli_text(
        "{.strong Included Columns:} {paste(included_cols, collapse = ', ')}"
      )
    }
    if (length(dropped_cols) > 0) {
      cli::cli_text(
        "{.strong Dropped Columns:} {paste(dropped_cols, collapse = ', ')}"
      )
    }
    if (length(aliased_cols) > 0) {
      cli::cli_text(
        "{.strong Aliased Columns:} {paste(aliased_cols, collapse = ', ')}"
      )
    }
  }
}

#' Print THETA parameters
#'
#' @param x A hyperion_nonmem_model object
#' @param digits Number of significant digits (uses global option if NULL)
#' @param theta_names Character vector of THETA parameter names from pharos
#' @return NULL (prints to console)
#' @keywords internal
#' @noRd
print_theta_parameters <- function(x, digits = NULL, theta_names) {
  formatted_data <- get_theta_parameter_data(x, digits, theta_names)
  if (!is.null(formatted_data)) {
    print_data_table_console(formatted_data, "Theta Parameters")
  }
}

#' Print OMEGA parameters using pre-computed names
#'
#' @param x A hyperion_nonmem_model object
#' @param digits Number of significant digits (uses global option if NULL)
#' @param omega_names Character vector of OMEGA parameter names from pharos
#' @return NULL (prints to console)
#' @keywords internal
#' @noRd
print_omega_parameters <- function(x, digits = NULL, omega_names) {
  formatted_data <- get_random_effect_parameter_data(
    x$omega_blocks,
    digits,
    omega_names
  )
  if (!is.null(formatted_data)) {
    print_data_table_console(formatted_data, "Omega Parameters")
  }
}

#' Print SIGMA parameters using pre-computed names
#'
#' @param x A hyperion_nonmem_model object
#' @param digits Number of significant digits (uses global option if NULL)
#' @param sigma_names Character vector of SIGMA parameter names from pharos
#' @return NULL (prints to console)
#' @keywords internal
#' @noRd
print_sigma_parameters <- function(x, digits = NULL, sigma_names) {
  formatted_data <- get_random_effect_parameter_data(
    x$sigma_blocks,
    digits,
    sigma_names
  )
  if (!is.null(formatted_data)) {
    print_data_table_console(formatted_data, "Sigma Parameters")
  }
}


#' Create BlockSame parameter data frame
#'
#' @param param_names Character vector of parameter names for this BlockSame
#' @param prev_block Previous Block to copy values from
#' @return Data frame with BlockSame parameters
#' @keywords internal
#' @noRd
create_blocksame_data <- function(param_names, prev_block) {
  # BlockSame copies everything from the previous Block's parameters
  # but uses new parameter names (e.g., OMEGA(8,8) instead of OMEGA(7,7))
  data.frame(
    Parameter = param_names,
    Initial = sapply(
      prev_block$parameters,
      function(p) p$initial_value %||% NA
    ),
    Lower = sapply(prev_block$parameters, function(p) p$lower_bound %||% NA),
    Upper = sapply(prev_block$parameters, function(p) p$upper_bound %||% NA),
    Fixed = sapply(
      prev_block$parameters,
      function(p) ifelse(p$is_fixed %||% FALSE, "Yes", "No")
    ),
    Parametrization = rep(
      prev_block$parametrization %||% "",
      length(param_names)
    ),
    Comment = sapply(prev_block$parameters, function(p) p$comment %||% ""),
    stringsAsFactors = FALSE
  )
}


#' Get formatted theta parameter data (shared by console and knit functions)
#'
#' @param x A hyperion_nonmem_model object
#' @param digits Number of significant digits (uses global option if NULL)
#' @param theta_names Character vector of THETA parameter names from pharos
#' @return Formatted data frame or NULL if no parameters
#' @keywords internal
#' @noRd
get_theta_parameter_data <- function(x, digits = NULL, theta_names) {
  if (is.null(x$theta_parameters) || length(x$theta_parameters) == 0) {
    return(NULL)
  }

  # Build parameter table
  param_data <- data.frame(
    Parameter = theta_names,
    Initial = sapply(x$theta_parameters, function(p) p$initial_value %||% NA),
    Lower = sapply(x$theta_parameters, function(p) p$lower_bound %||% NA),
    Upper = sapply(x$theta_parameters, function(p) p$upper_bound %||% NA),
    Fixed = sapply(
      x$theta_parameters,
      function(p) ifelse(p$is_fixed %||% FALSE, "Yes", "No")
    ),
    Comment = sapply(x$theta_parameters, function(p) p$comment %||% ""),
    stringsAsFactors = FALSE
  )

  # Use unified formatting
  format_display_data(param_data, digits)
}

#' Get formatted random effect parameter data (shared by omega and sigma functions)
#'
#' @param blocks List of parameter blocks (omega_blocks or sigma_blocks)
#' @param digits Number of significant digits (uses global option if NULL)
#' @param param_names Character vector of parameter names from pharos
#' @return Formatted data frame or NULL if no parameters
#' @keywords internal
#' @noRd
get_random_effect_parameter_data <- function(
  blocks,
  digits = NULL,
  param_names
) {
  if (is.null(blocks) || length(blocks) == 0) {
    return(NULL)
  }

  param_idx <- 1
  all_param_data <- data.frame()

  for (i in seq_along(blocks)) {
    block <- blocks[[i]]

    # Handle blocks with parameters
    if (!is.null(block$parameters) && length(block$parameters) > 0) {
      num_params <- length(block$parameters)

      block_data <- data.frame(
        Parameter = param_names[param_idx:(param_idx + num_params - 1)],
        Initial = sapply(block$parameters, function(p) p$initial_value %||% NA),
        Lower = sapply(block$parameters, function(p) p$lower_bound %||% NA),
        Upper = sapply(block$parameters, function(p) p$upper_bound %||% NA),
        Fixed = sapply(
          block$parameters,
          function(p) ifelse(p$is_fixed %||% FALSE, "Yes", "No")
        ),
        Parametrization = rep(block$parametrization %||% "", num_params),
        Comment = sapply(block$parameters, function(p) p$comment %||% ""),
        stringsAsFactors = FALSE
      )

      all_param_data <- rbind(all_param_data, block_data)
      param_idx <- param_idx + num_params
    } else {
      # Handle BlockSame and other cases
      if (is.list(block$structure) && !is.null(block$structure$BlockSame)) {
        # BlockSame refers to the most recent Block with the same size
        block_same_size <- block$structure$BlockSame$size
        prev_block <- NULL
        for (j in (i - 1):1) {
          struct_j <- blocks[[j]]$structure
          if (
            is.list(struct_j) &&
              "Block" %in% names(struct_j) &&
              struct_j$Block$size == block_same_size
          ) {
            prev_block <- blocks[[j]]
            break
          }
        }

        if (is.null(prev_block)) {
          stop(
            "BlockSame found but no previous Block structure with matching size exists."
          )
        }

        num_params <- length(prev_block$parameters)

        # Get parameter names for this block
        block_param_names <- param_names[param_idx:(param_idx + num_params - 1)]

        # Create parameter data using previous block's structure
        block_data <- create_blocksame_data(
          block_param_names,
          prev_block
        )

        all_param_data <- rbind(all_param_data, block_data)
        param_idx <- param_idx + num_params
      } else if (
        is.list(block$structure) && "Block" %in% names(block$structure)
      ) {
        # Regular block without parameters - just advance index
        param_idx <- param_idx + block$structure$Block$size
      } else if (identical(block$structure, "Diagonal")) {
        # Diagonal block without parameters - just advance index
        param_idx <- param_idx + 1
      } else {
        # Other block types - fallback
        param_idx <- param_idx + 1
      }
    }
  }

  if (nrow(all_param_data) == 0) {
    return(NULL)
  }

  # Use unified formatting
  format_display_data(all_param_data, digits)
}

#' Format a single IGNORE condition for display
#'
#' @param ignore_obj A single ignore object from x$data$ignore list
#' @return Character string representation of the ignore condition
#' @keywords internal
#' @noRd
format_ignore_condition <- function(ignore_obj) {
  if (!is.null(ignore_obj$Marker)) {
    return(ignore_obj$Marker)
  } else if (!is.null(ignore_obj$ValueFilter)) {
    # Format ValueFilter as field.op.value (e.g., "AN01FL.EQ.0")
    field <- ignore_obj$ValueFilter$field %||% "Unknown"
    op <- ignore_obj$ValueFilter$op %||% "Unknown"
    value <- ignore_obj$ValueFilter$value %||% "Unknown"

    # Convert operation names to NONMEM-style operators
    op_map <- c(
      "Equal" = "EQ",
      "NotEqual" = "NE",
      "Greater" = "GT",
      "GreaterEqual" = "GE",
      "Less" = "LT",
      "LessEqual" = "LE"
    )
    op_symbol <- op_map[op] %||% op

    return(paste0(field, ".", op_symbol, ".", value))
  } else {
    return("Unknown")
  }
}

#' Knit print method for hyperion_nonmem_model objects (for Quarto/R Markdown)
#' @param x A hyperion_nonmem_model object
#' @param ... Additional arguments (ignored)
#' @return HTML/markdown output for rendered documents
#' @exportS3Method knitr::knit_print
knit_print.hyperion_nonmem_model <- function(x, ...) {
  # Build markdown output
  output <- character()

  # Header with filename if available
  if (!is.null(x$filename)) {
    output <- c(output, paste0("# NONMEM Model: ", x$filename), "")
  } else {
    output <- c(output, "# NONMEM Model", "")
  }

  # Problem information
  if (!is.null(x$problem)) {
    if (is.character(x$problem) && length(x$problem) > 0) {
      output <- c(output, paste0("**Problem:** ", x$problem))
    } else if (is.list(x$problem) && !is.null(x$problem$title)) {
      output <- c(output, paste0("**Problem:** ", x$problem$title))
    }
  }

  # Record information
  if (!is.null(x$records)) {
    output <- c(
      output,
      paste0("**Records:** ", length(x$records), " record blocks")
    )

    # Count record types
    if (length(x$records) > 0) {
      record_types <- sapply(x$records, function(r) {
        if (is.list(r) && !is.null(r$record_type)) {
          r$record_type
        } else {
          "Unknown"
        }
      })
      record_counts <- table(record_types)

      output <- c(output, "**Record Types:**")
      for (i in seq_along(record_counts)) {
        type <- names(record_counts)[i]
        count <- record_counts[i]
        output <- c(output, paste0("- ", type, ": ", count))
      }
    }
  }
  output <- c(output, "")

  # Dataset and input columns information
  output <- c(output, knit_print_model_data_info(x))

  # Get all parameter names once from pharos
  all_param_names <- get_model_parameter_names(x)

  # Extract names by parameter type
  theta_names <- names(all_param_names)[grepl("^THETA", names(all_param_names))]
  omega_names <- names(all_param_names)[grepl("^OMEGA", names(all_param_names))]
  sigma_names <- names(all_param_names)[grepl("^SIGMA", names(all_param_names))]

  # Parameter sections
  output <- c(output, knit_print_theta_parameters(x, theta_names))
  output <- c(output, knit_print_omega_parameters(x, omega_names))
  output <- c(output, knit_print_sigma_parameters(x, sigma_names))

  # Return as HTML
  knitr::asis_output(paste(output, collapse = "\n"))
}

#' Knit print model data and input column information
#' @param x A hyperion_nonmem_model object
#' @return Character vector of markdown lines
#' @keywords internal
#' @noRd
knit_print_model_data_info <- function(x) {
  output <- character()

  # Dataset information
  if (!is.null(x$data)) {
    if (is.character(x$data) && length(x$data) > 0) {
      output <- c(output, paste0("**Dataset:** ", x$data), "")
    } else if (is.list(x$data)) {
      if (!is.null(x$data$path)) {
        output <- c(output, paste0("**Dataset:** ", x$data$path), "")
      }

      # Show ignore conditions if any
      if (!is.null(x$data$ignore) && length(x$data$ignore) > 0) {
        ignore_markers <- sapply(x$data$ignore, format_ignore_condition)
        output <- c(
          output,
          paste0("**Ignore:** ", paste(ignore_markers, collapse = ", ")),
          ""
        )
      }

      # Show number of records if available
      if (!is.null(x$data$num_records)) {
        output <- c(output, paste0("**Records:** ", x$data$num_records), "")
      }
    }
  }

  # Input columns information
  if (!is.null(x$input_columns) && length(x$input_columns) > 0) {
    # Handle different column types (Included, Dropped, Aliased)
    included_cols <- c()
    dropped_cols <- c()
    aliased_cols <- c()

    for (col in x$input_columns) {
      if (!is.null(col$Included)) {
        included_cols <- c(included_cols, col$Included)
      } else if (!is.null(col$Dropped)) {
        dropped_cols <- c(dropped_cols, col$Dropped)
      } else if (!is.null(col$Aliased)) {
        aliased_cols <- c(
          aliased_cols,
          paste0(col$Aliased$from, " \u2192 ", col$Aliased$to)
        )
      }
    }

    if (
      length(included_cols) > 0 &&
        getOption("hyperion.nonmem_model.show_included_columns", FALSE)
    ) {
      output <- c(
        output,
        paste0("**Included Columns:** ", paste(included_cols, collapse = ", ")),
        ""
      )
    }
    if (length(dropped_cols) > 0) {
      output <- c(
        output,
        paste0("**Dropped Columns:** ", paste(dropped_cols, collapse = ", ")),
        ""
      )
    }
    if (length(aliased_cols) > 0) {
      output <- c(
        output,
        paste0("**Aliased Columns:** ", paste(aliased_cols, collapse = ", ")),
        ""
      )
    }
  }

  return(output)
}

#' Knit print THETA parameters
#' @param x A hyperion_nonmem_model object
#' @param theta_names Character vector of THETA parameter names from pharos
#' @return Character vector of markdown lines
#' @keywords internal
#' @noRd
knit_print_theta_parameters <- function(x, theta_names) {
  formatted_data <- get_theta_parameter_data(x, NULL, theta_names)
  if (!is.null(formatted_data)) {
    return(print_data_table_knit(formatted_data, "Theta Parameters"))
  }
  return(character())
}

#' Knit print OMEGA parameters
#' @param x A hyperion_nonmem_model object
#' @param omega_names Character vector of OMEGA parameter names from pharos
#' @return Character vector of markdown lines
#' @keywords internal
#' @noRd
knit_print_omega_parameters <- function(x, omega_names) {
  formatted_data <- get_random_effect_parameter_data(
    x$omega_blocks,
    NULL,
    omega_names
  )
  if (!is.null(formatted_data)) {
    return(print_data_table_knit(formatted_data, "Omega Parameters"))
  }
  return(character())
}

#' Knit print SIGMA parameters
#' @param x A hyperion_nonmem_model object
#' @param sigma_names Character vector of SIGMA parameter names from pharos
#' @return Character vector of markdown lines
#' @keywords internal
#' @noRd
knit_print_sigma_parameters <- function(x, sigma_names) {
  formatted_data <- get_random_effect_parameter_data(
    x$sigma_blocks,
    NULL,
    sigma_names
  )
  if (!is.null(formatted_data)) {
    return(print_data_table_knit(formatted_data, "Sigma Parameters"))
  }
  return(character())
}
