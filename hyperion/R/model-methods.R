#' Print method for hyperion_model objects
#'
#' @param x A hyperion_model object
#' @param ... Additional arguments (ignored)
#' @return Invisible copy of x
#' @export
print.hyperion_model <- function(x, ...) {
  print_model_header(x)
  print_model_data_info(x)
  print_theta_parameters(x)
  print_omega_parameters(x)
  print_sigma_parameters(x)

  invisible(x)
}

#' Print model header information
#'
#' @param x A hyperion_model object
#' @return NULL (prints to console)
#' @keywords internal
#' @noRd
print_model_header <- function(x) {
  cli::cli_h1("NONMEM Model")

  # Problem information - handle different possible structures
  if (!is.null(x$problem)) {
    if (is.character(x$problem) && length(x$problem) > 0) {
      cli::cli_text("{.strong Problem:} {x$problem}")
    } else if (is.list(x$problem) && !is.null(x$problem$title)) {
      cli::cli_text("{.strong Problem:} {x$problem$title}")
    }
  }

  # Model file information
  if (!is.null(x$filename)) {
    cli::cli_text("{.strong File:} {x$filename}")
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
        cli::cli_text("  • {type}: {count}")
      }
    }
  }
}

#' Print model data and input column information
#'
#' @param x A hyperion_model object
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
        cli::cli_text("{.strong Ignore:} {paste(ignore_markers, collapse = ', ')}")
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
        aliased_cols <- c(aliased_cols, paste0(col$Aliased$from, "→", col$Aliased$to))
      }
    }

    if (length(included_cols) > 0) {
      cli::cli_text("{.strong Included Columns:} {paste(included_cols, collapse = ', ')}")
    }
    if (length(dropped_cols) > 0) {
      cli::cli_text("{.strong Dropped Columns:} {paste(dropped_cols, collapse = ', ')}")
    }
    if (length(aliased_cols) > 0) {
      cli::cli_text("{.strong Aliased Columns:} {paste(aliased_cols, collapse = ', ')}")
    }
  }
}

#' Print THETA parameters
#'
#' @param x A hyperion_model object
#' @return NULL (prints to console)
#' @keywords internal
#' @noRd
print_theta_parameters <- function(x) {
  if (!is.null(x$theta_parameters) && length(x$theta_parameters) > 0) {
    cli::cli_h2("Theta Parameters")

    # Build parameter table
    param_data <- data.frame(
      Parameter = paste0("THETA", seq_along(x$theta_parameters)),
      Initial = sapply(x$theta_parameters, function(p) p$initial_value %||% NA),
      Lower = sapply(x$theta_parameters, function(p) p$lower_bound %||% NA),
      Upper = sapply(x$theta_parameters, function(p) p$upper_bound %||% NA),
      Fixed = sapply(x$theta_parameters, function(p) ifelse(p$is_fixed %||% FALSE, "yes", "no")),
      Comment = sapply(x$theta_parameters, function(p) p$comment %||% ""),
      stringsAsFactors = FALSE
    )
    cli::cat_line(" ")
    format_parameter_table_unified(param_data)
  }
}

#' Print OMEGA parameters using generic block processor
#'
#' @param x A hyperion_model object
#' @return NULL (prints to console)
#' @keywords internal
#' @noRd
print_omega_parameters <- function(x) {
  if (!is.null(x$omega_blocks) && length(x$omega_blocks) > 0) {
    cli::cli_h2("Omega Parameters")

    all_omega_data <- process_parameter_blocks(
      x$omega_blocks,
      "OMEGA",
      generate_omega_names
    )

    cli::cat_line(" ")
    format_parameter_table_unified(all_omega_data)
  }
}

#' Print SIGMA parameters using generic block processor
#'
#' @param x A hyperion_model object
#' @return NULL (prints to console)
#' @keywords internal
#' @noRd
print_sigma_parameters <- function(x) {
  if (!is.null(x$sigma_blocks) && length(x$sigma_blocks) > 0) {
    cli::cli_h2("Sigma Parameters")

    all_sigma_data <- process_parameter_blocks(
      x$sigma_blocks,
      "SIGMA",
      generate_sigma_names
    )

    cli::cat_line(" ")
    format_parameter_table_unified(all_sigma_data)
  }
}


#' Parse block structure information
#'
#' @param block A single block from omega_blocks or sigma_blocks
#' @return List with structure_info and block_size
#' @keywords internal
#' @noRd
parse_block_structure <- function(block) {
  structure_info <- "Unknown"
  block_size <- 1

  if (is.character(block$structure)) {
    structure_info <- block$structure
    if (structure_info == "Diagonal" && !is.null(block$parameters)) {
      block_size <- length(block$parameters)
    }
  } else if (is.list(block$structure)) {
    if (!is.null(block$structure$Block$size)) {
      structure_info <- paste0("Block(", block$structure$Block$size, ")")
      block_size <- block$structure$Block$size
    } else if (!is.null(block$structure$BlockSame$size)) {
      structure_info <- paste0("BlockSame(", block$structure$BlockSame$size, ")")
      block_size <- block$structure$BlockSame$size
    }
  }

  list(structure_info = structure_info, block_size = block_size)
}


#' Create BlockSame parameter data frame
#'
#' @param param_names Character vector of parameter names for current ETA range
#' @param prev_values Previous block values for copying
#' @param current_block Current block for comment and parametrization
#' @return Data frame with BlockSame parameters
#' @keywords internal
#' @noRd
create_blocksame_data <- function(param_names, prev_values, current_block) {
  if (length(prev_values$parameters) > 0) {
    # Copy values from previous block but use new parameter names
    data.frame(
      Parameter = param_names,
      Initial = sapply(prev_values$parameters, function(p) p$initial_value %||% NA),
      Lower = sapply(prev_values$parameters, function(p) p$lower_bound %||% NA),
      Upper = sapply(prev_values$parameters, function(p) p$upper_bound %||% NA),
      Fixed = sapply(prev_values$parameters, function(p) ifelse(p$is_fixed %||% FALSE, "yes", "no")),
      Parametrization = rep(prev_values$parametrization, length(param_names)),
      Comment = rep(current_block$comment %||% "", length(param_names)),
      stringsAsFactors = FALSE
    )
  } else {
    # Fallback if no previous block found
    data.frame(
      Parameter = param_names,
      Initial = rep(NA, length(param_names)),
      Lower = rep(NA, length(param_names)),
      Upper = rep(NA, length(param_names)),
      Fixed = rep("N/A", length(param_names)),
      Parametrization = rep(current_block$parametrization %||% "", length(param_names)),
      Comment = rep(current_block$comment %||% "", length(param_names)),
      stringsAsFactors = FALSE
    )
  }
}

#' Process parameter blocks generically for OMEGA or SIGMA
#'
#' @param blocks List of parameter blocks (omega_blocks or sigma_blocks)
#' @param param_type Character, either "OMEGA" or "SIGMA"
#' @param name_generator Function to generate parameter names (generate_omega_names or generate_sigma_names)
#' @return Data frame with all processed parameters
#' @keywords internal
#' @noRd
process_parameter_blocks <- function(blocks, param_type, name_generator) {
  index <- 1 # Track current parameter index across blocks
  all_param_data <- data.frame() # Collect all parameters

  for (i in seq_along(blocks)) {
    block <- blocks[[i]]
    parsed <- parse_block_structure(block)
    structure_info <- parsed$structure_info
    block_size <- parsed$block_size

    # Handle BlockSame or other blocks with no parameters
    if ((is.null(block$parameters) || length(block$parameters) == 0)) {
      if (grepl("BlockSame", structure_info) || (!is.null(block$structure$BlockSame))) {
        # BlockSame always refers to the immediately previous block
        if (i > 1) {
          prev_block <- blocks[[i - 1]]
          prev_values <- list(
            parameters = prev_block$parameters,
            parametrization = prev_block$parametrization %||% "",
            structure_info = parse_block_structure(prev_block)$structure_info
          )
        } else {
          # Shouldn't happen, but fallback
          prev_values <- list(parameters = list(), parametrization = "", structure_info = "Block")
        }

        # For BlockSame, generate parameter names for the current ETA range
        # Use the structure info from the previous block to generate correct names
        if (length(prev_values$parameters) > 0) {
          expected_params <- length(prev_values$parameters)
          param_names <- name_generator(prev_values$structure_info, index, block_size, expected_params)
        } else {
          # Fallback if no previous block found - assume block structure
          expected_params <- block_size * (block_size + 1) / 2 # Lower triangular matrix size
          param_names <- name_generator("Block", index, block_size, expected_params)
        }

        # Create data frame copying values from previous block but with new names
        blocksame_data <- create_blocksame_data(param_names, prev_values, block)
        all_param_data <- rbind(all_param_data, blocksame_data)
      } else if (!is.null(block$comment) && length(block$comment) > 0) {
        # For other blocks with comments but no parameters, create a single note row
        note_data <- data.frame(
          Parameter = paste0(param_type, "(", index, ":", index + block_size - 1, ")"),
          Initial = NA,
          Lower = NA,
          Upper = NA,
          Fixed = "N/A",
          Parametrization = block$parametrization %||% "",
          Comment = block$comment,
          stringsAsFactors = FALSE
        )
        all_param_data <- rbind(all_param_data, note_data)
      }
      index <- index + block_size # Still advance parameter index
    } else if (!is.null(block$parameters) && length(block$parameters) > 0) {
      # Generate parameter names based on block structure
      param_names <- name_generator(structure_info, index, block_size, length(block$parameters))

      block_param_data <- data.frame(
        Parameter = param_names,
        Initial = sapply(block$parameters, function(p) p$initial_value %||% NA),
        Lower = sapply(block$parameters, function(p) p$lower_bound %||% NA),
        Upper = sapply(block$parameters, function(p) p$upper_bound %||% NA),
        Fixed = sapply(block$parameters, function(p) ifelse(p$is_fixed %||% FALSE, "yes", "no")),
        Parametrization = rep(block$parametrization %||% "", length(param_names)),
        Comment = sapply(block$parameters, function(p) p$comment %||% ""),
        stringsAsFactors = FALSE
      )

      # Accumulate all parameters
      all_param_data <- rbind(all_param_data, block_param_data)

      index <- index + block_size # Advance parameter index by block size
    } else {
      index <- index + block_size # Still advance even if no parameters
    }
  }

  return(all_param_data)
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

