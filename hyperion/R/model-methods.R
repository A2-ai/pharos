#' Print method for hyperion_nonmem_model objects
#'
#' @param x A hyperion_nonmem_model object
#' @param digits Number of significant digits (uses global option if NULL)
#' @param ... Additional arguments (ignored)
#' @return Invisible copy of x
#' @rawNamespace S3method(base::print, hyperion_nonmem_model)
print.hyperion_nonmem_model <- function(x, digits = NULL, ...) {
  parts <- build_model_display_parts(x, digits)

  cli::cli_text("")
  cli::cli_h1(parts$title)

  if (!is.null(parts$problem)) {
    cli::cli_text("{.strong Problem:} {parts$problem}")
  }
  if (!is.null(parts$run_status)) {
    cli::cli_text("{.strong Run Status:} {parts$run_status}")
  }

  if (!is.null(parts$records)) {
    cli::cli_text("{.strong Records:} {parts$records$count} record blocks")
    if (length(parts$records$types) > 0) {
      cli::cli_text("{.strong Record Types:}")
      for (i in seq_along(parts$records$types)) {
        type <- names(parts$records$types)[i]
        count <- parts$records$types[i]
        cli::cli_text("  \u2022 {type}: {count}")
      }
    }
  }

  if (!is.null(parts$data$dataset)) {
    cli::cli_text("{.strong Dataset:} {parts$data$dataset}")
  }
  if (!is.null(parts$data$ignore) && length(parts$data$ignore) > 0) {
    cli::cli_text(
      "{.strong Ignore:} {paste(parts$data$ignore, collapse = ', ')}"
    )
  }
  if (!is.null(parts$data$num_records)) {
    cli::cli_text("{.strong Records:} {parts$data$num_records}")
  }

  if (!is.null(parts$input_columns)) {
    if (
      length(parts$input_columns$included) > 0 &&
        getOption("hyperion.nonmem_model.show_included_columns", FALSE)
    ) {
      cli::cli_text(
        "{.strong Included Columns:} {paste(parts$input_columns$included, collapse = ', ')}"
      )
    }
    if (length(parts$input_columns$dropped) > 0) {
      cli::cli_text(
        "{.strong Dropped Columns:} {paste(parts$input_columns$dropped, collapse = ', ')}"
      )
    }
    if (nrow(parts$input_columns$aliased) > 0) {
      aliased <- paste0(
        parts$input_columns$aliased$from,
        "\u2192",
        parts$input_columns$aliased$to
      )
      cli::cli_text(
        "{.strong Aliased Columns:} {paste(aliased, collapse = ', ')}"
      )
    }
  }

  if (!is.null(parts$tables$theta)) {
    print_data_table_console(parts$tables$theta$data, parts$tables$theta$title)
  }
  if (!is.null(parts$tables$omega)) {
    print_data_table_console(parts$tables$omega$data, parts$tables$omega$title)
  }
  if (!is.null(parts$tables$sigma)) {
    print_data_table_console(parts$tables$sigma$data, parts$tables$sigma$title)
  }

  invisible(x)
}

#' Summary method for hyperion_nonmem_model objects
#'
#' @param object A hyperion_nonmem_model object
#' @param hide_off_diagonal_params Logical, if TRUE will not display the unfixed
#'   off-diagonal estimated parameters. Default is FALSE.
#' @param n_iterations Number of recent iterations to show for running models.
#'   Default is 10.
#' @param ... Additional arguments (currently unused)
#' @return A hyperion_nonmem_summary object
#' @rawNamespace S3method(base::summary, hyperion_nonmem_model)
summary.hyperion_nonmem_model <- function(
  object,
  hide_off_diagonal_params = FALSE,
  n_iterations = 10,
  ...
) {
  validate_n_iterations(n_iterations)

  run_status <- refresh_run_status(object)

  # Handle "not_run" status - return informative summary instead of error
  if (identical(run_status, "not_run")) {
    return(build_not_run_summary(object))
  }

  if (identical(run_status, "running")) {
    return(build_running_summary(object, n_iterations))
  }

  if (!identical(run_status, "run")) {
    rlang::abort(paste0(
      "model run_status must be 'run', 'running', or 'not_run', got: ",
      run_status
    ))
  }

  summary_obj <- get_model_summary_internal(
    object,
    hide_off_diagonal_params = hide_off_diagonal_params
  )

  comment_type <- get_comment_type()
  is_type1 <- !is.null(comment_type) &&
    is.character(comment_type) &&
    length(comment_type) == 1 &&
    identical(tolower(comment_type), "type1")

  if (!is_type1 && !is.null(summary_obj$parameters)) {
    info <- get_model_parameter_info(object)
    name_map <- get_parameter_names(info)

    if (nrow(name_map) > 0 && "name" %in% names(summary_obj$parameters)) {
      nonmem_names <- summary_obj$parameters$name
      mapped <- name_map[nonmem_names, "name", drop = TRUE]
      replace_idx <- !is.na(mapped) & nzchar(mapped)
      summary_obj$parameters$name[replace_idx] <- mapped[replace_idx]
    }
  }

  summary_obj
}

#' Build summary object for a running model
#'
#' @param object A hyperion_nonmem_model object
#' @param n_iterations Number of recent iterations to include
#' @return A hyperion_nonmem_summary object with iterations and gradients
#' @keywords internal
#' @noRd
build_running_summary <- function(object, n_iterations) {
  run_name <- get_model_name(object)
  model_path <- from_config_relative(attr(object, "model_source"))

  # Get recent iterations from ext file (may not exist yet)
  iterations <- tryCatch(
    {
      ext_data <- read_ext_file(
        model_path,
        parameters_only = TRUE,
        only_last = TRUE
      )
      if (!is.null(ext_data) && nrow(ext_data) > 0) {
        n_rows <- nrow(ext_data)
        start_row <- max(1, n_rows - n_iterations + 1)
        ext_data[start_row:n_rows, , drop = FALSE]
      } else {
        NULL
      }
    },
    error = function(e) NULL
  )

  # Get gradients from grd file (may not exist yet)
  gradients <- tryCatch(
    {
      grd_data <- get_gradients(object, only_last = TRUE)
      if (!is.null(grd_data) && nrow(grd_data) > 0) {
        n_rows <- nrow(grd_data)
        start_row <- max(1, n_rows - n_iterations + 1)
        grd_data[start_row:n_rows, , drop = FALSE]
      } else {
        NULL
      }
    },
    error = function(e) NULL
  )

  summary_obj <- list(
    run_name = run_name,
    run_status = "running",
    iterations = iterations,
    gradients = gradients
  )

  class(summary_obj) <- c(
    "hyperion_nonmem_summary_running",
    "hyperion_nonmem_summary"
  )
  summary_obj
}

#' Build summary object for a model that has not been run
#'
#' @param object A hyperion_nonmem_model object
#' @return A hyperion_nonmem_summary object with basic model info
#' @keywords internal
#' @noRd
build_not_run_summary <- function(object) {
  run_name <- get_model_name(object)

  problem <- NULL
  if (!is.null(object$problem)) {
    if (is.character(object$problem) && length(object$problem) > 0) {
      problem <- object$problem
    } else if (is.list(object$problem) && !is.null(object$problem$title)) {
      problem <- object$problem$title
    }
  }

  data_path <- NULL
  if (!is.null(object$data)) {
    if (is.character(object$data) && length(object$data) > 0) {
      data_path <- object$data
    } else if (is.list(object$data) && !is.null(object$data$path)) {
      data_path <- object$data$path
    }
  }

  summary_obj <- list(
    run_name = run_name,
    run_status = "not_run",
    problem = problem,
    data_path = data_path
  )

  class(summary_obj) <- c(
    "hyperion_nonmem_summary_not_run",
    "hyperion_nonmem_summary"
  )
  summary_obj
}

#' Gets model run summary
#'
#' @description
#' `r lifecycle::badge("deprecated")`
#'
#' `get_model_summary()` was deprecated in hyperion 0.2.0 in favor of
#' `summary(model)`. It was removed in hyperion 0.3.0.
#'
#' @param directory path to model run output directory containing .ext, .lst
#'   files, or a hyperion_nonmem_model object
#' @param hide_off_diagonal_params boolean, if TRUE will not display the unfixed
#'   off-diagonal estimated parameters
#'
#' @return hyperion_nonmem_summary S3 object
#' @export
get_model_summary <- function(directory, hide_off_diagonal_params = FALSE) {
  lifecycle::deprecate_stop(
    "0.3.0",
    "get_model_summary()",
    "summary()",
    details = "`get_model_summary()` was removed in hyperion 0.3.0. Use `summary(model)` instead."
  )
}

#' @keywords internal
#' @noRd
validate_n_iterations <- function(n_iterations) {
  if (
    !is.numeric(n_iterations) ||
      length(n_iterations) != 1 ||
      is.na(n_iterations) ||
      !is.finite(n_iterations) ||
      n_iterations <= 0 ||
      n_iterations != as.integer(n_iterations)
  ) {
    rlang::abort(
      paste0(
        "`n_iterations` must be a single positive integer, got: ",
        deparse(n_iterations)
      )
    )
  }

  invisible(n_iterations)
}

#' Structure method for hyperion_nonmem_model objects
#'
#' Displays the structure of a model object, excluding verbose token fields.
#'
#' @param object A hyperion_nonmem_model object
#' @param ... Additional arguments passed to str
#' @return Invisible NULL (called for side effects)
#' @rawNamespace S3method(utils::str, hyperion_nonmem_model)
str.hyperion_nonmem_model <- function(object, ...) {
  class(object) <- "list"
  object$tokens <- NULL
  object$token_ranges <- NULL
  utils::str(object, ...)
}

#' Element access for hyperion_nonmem_model objects
#'
#' Prevents direct access to internal token fields.
#'
#' @param x A hyperion_nonmem_model object
#' @param name The element name to access
#' @return The element value, or NULL for restricted fields
#' @rawNamespace S3method(base::`$`, hyperion_nonmem_model)
`$.hyperion_nonmem_model` <- function(x, name) {
  if (name %in% c("tokens", "token_ranges")) {
    return(NULL)
  }
  .subset2(x, name)
}

#' @rawNamespace S3method(base::`[[`, hyperion_nonmem_model)
`[[.hyperion_nonmem_model` <- function(x, i, ...) {
  if (is.character(i) && i %in% c("tokens", "token_ranges")) {
    return(NULL)
  }
  NextMethod("[[")
}

#' @rawNamespace S3method(base::names, hyperion_nonmem_model)
names.hyperion_nonmem_model <- function(x) {
  n <- NextMethod("names")
  setdiff(n, c("tokens", "token_ranges"))
}

#' @noRd
build_model_display_parts <- function(x, digits = NULL) {
  filename <- attr(x, "filename")
  title <- if (!is.null(filename)) {
    paste0("NONMEM Model: ", filename)
  } else {
    "NONMEM Model"
  }

  problem <- NULL
  if (!is.null(x$problem)) {
    if (is.character(x$problem) && length(x$problem) > 0) {
      problem <- x$problem
    } else if (is.list(x$problem) && !is.null(x$problem$title)) {
      problem <- x$problem$title
    }
  }

  run_status <- format_run_status(refresh_run_status(x))

  records <- NULL
  if (!is.null(x$records)) {
    if (length(x$records) > 0) {
      record_types <- sapply(x$records, function(r) {
        if (is.list(r) && !is.null(r$record_type)) {
          r$record_type
        } else {
          NA_character_
        }
      })
    } else {
      record_types <- character(0)
    }
    records <- list(
      count = length(x$records),
      types = table(record_types)
    )
  }

  data_info <- list(dataset = NULL, ignore = NULL, num_records = NULL)
  if (!is.null(x$data)) {
    if (is.character(x$data) && length(x$data) > 0) {
      data_info$dataset <- x$data
    } else if (is.list(x$data)) {
      if (!is.null(x$data$path)) {
        data_info$dataset <- x$data$path
      }
      if (!is.null(x$data$ignore) && length(x$data$ignore) > 0) {
        data_info$ignore <- sapply(x$data$ignore, format_ignore_condition)
      }
      if (!is.null(x$data$num_records)) {
        data_info$num_records <- x$data$num_records
      }
    }
  }

  input_columns <- NULL
  if (!is.null(x$input_columns) && length(x$input_columns) > 0) {
    included_cols <- c()
    dropped_cols <- c()
    aliased_cols <- data.frame(
      from = character(0),
      to = character(0),
      stringsAsFactors = FALSE
    )

    for (col in x$input_columns) {
      if (!is.null(col$Included)) {
        included_cols <- c(included_cols, col$Included)
      } else if (!is.null(col$Dropped)) {
        dropped_cols <- c(dropped_cols, col$Dropped)
      } else if (!is.null(col$Aliased)) {
        aliased_cols <- rbind(
          aliased_cols,
          data.frame(
            from = col$Aliased$from,
            to = col$Aliased$to,
            stringsAsFactors = FALSE
          )
        )
      }
    }

    input_columns <- list(
      included = included_cols,
      dropped = dropped_cols,
      aliased = aliased_cols
    )
  }

  all_param_names <- get_model_parameter_names(x)
  param_names <- names(all_param_names)
  theta_names <- param_names[grepl("^THETA", param_names)]
  omega_names <- param_names[grepl("^OMEGA", param_names)]
  sigma_names <- param_names[grepl("^SIGMA", param_names)]

  tables <- list(
    theta = list(
      title = "Theta Parameters",
      data = get_theta_parameter_data(x, digits, theta_names)
    ),
    omega = list(
      title = "Omega Parameters",
      data = get_random_effect_parameter_data(
        x$omega_blocks,
        digits,
        omega_names
      )
    ),
    sigma = list(
      title = "Sigma Parameters",
      data = get_random_effect_parameter_data(
        x$sigma_blocks,
        digits,
        sigma_names
      )
    )
  )
  tables <- Filter(function(item) !is.null(item$data), tables)

  list(
    title = title,
    problem = problem,
    run_status = run_status,
    records = records,
    data = data_info,
    input_columns = input_columns,
    tables = tables
  )
}

#' @noRd
format_run_status <- function(run_status) {
  if (
    is.null(run_status) || !is.character(run_status) || length(run_status) == 0
  ) {
    return(NULL)
  }
  if (!nzchar(run_status)) {
    return(NULL)
  }
  tools::toTitleCase(gsub("_", " ", run_status))
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
        if (i > 1) {
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
        }

        if (is.null(prev_block)) {
          rlang::abort(
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
    field <- ignore_obj$ValueFilter$field %||% NA_character_
    op <- ignore_obj$ValueFilter$op %||% NA_character_
    value <- ignore_obj$ValueFilter$value %||% NA_character_

    # Convert operation names to NONMEM-style operators
    op_map <- c(
      "Equal" = "EQ",
      "NotEqual" = "NE",
      "Greater" = "GT",
      "GreaterEqual" = "GE",
      "Less" = "LT",
      "LessEqual" = "LE"
    )
    op_symbol <- op_map[op]
    if (is.na(op_symbol) || is.null(op_symbol)) {
      op_symbol <- op
    }

    return(paste0(field, ".", op_symbol, ".", value))
  } else {
    return(NA_character_)
  }
}

#' Knit print method for hyperion_nonmem_model objects (for Quarto/R Markdown)
#' @param x A hyperion_nonmem_model object
#' @param ... Additional arguments (ignored)
#' @return HTML/markdown output for rendered documents
#' @exportS3Method knitr::knit_print
knit_print.hyperion_nonmem_model <- function(x, ...) {
  parts <- build_model_display_parts(x)
  output <- character()

  output <- c(
    output,
    "",
    paste0("<strong>", parts$title, "</strong>"),
    ""
  )

  if (!is.null(parts$problem)) {
    output <- c(
      output,
      paste0("<strong>Problem:</strong> ", parts$problem),
      ""
    )
  }

  if (!is.null(parts$run_status)) {
    output <- c(
      output,
      paste0("<strong>Run Status:</strong> ", parts$run_status),
      ""
    )
  }

  if (!is.null(parts$records)) {
    output <- c(
      output,
      paste0(
        "<strong>Records:</strong> ",
        parts$records$count,
        " record blocks"
      )
    )
    if (length(parts$records$types) > 0) {
      output <- c(output, "<strong>Record Types:</strong>")
      for (i in seq_along(parts$records$types)) {
        type <- names(parts$records$types)[i]
        count <- parts$records$types[i]
        output <- c(output, paste0("- ", type, ": ", count))
      }
    }
  }
  output <- c(output, "")

  if (!is.null(parts$data$dataset)) {
    output <- c(
      output,
      paste0("<strong>Dataset:</strong> ", parts$data$dataset),
      ""
    )
  }
  if (!is.null(parts$data$ignore) && length(parts$data$ignore) > 0) {
    output <- c(
      output,
      paste0(
        "<strong>Ignore:</strong> ",
        paste(parts$data$ignore, collapse = ", ")
      ),
      ""
    )
  }
  if (!is.null(parts$data$num_records)) {
    output <- c(
      output,
      paste0("<strong>Records:</strong> ", parts$data$num_records),
      ""
    )
  }

  if (!is.null(parts$input_columns)) {
    if (
      length(parts$input_columns$included) > 0 &&
        getOption("hyperion.nonmem_model.show_included_columns", FALSE)
    ) {
      output <- c(
        output,
        paste0(
          "<strong>Included Columns:</strong> ",
          paste(parts$input_columns$included, collapse = ", ")
        ),
        ""
      )
    }
    if (length(parts$input_columns$dropped) > 0) {
      output <- c(
        output,
        paste0(
          "<strong>Dropped Columns:</strong> ",
          paste(parts$input_columns$dropped, collapse = ", ")
        ),
        ""
      )
    }
    if (nrow(parts$input_columns$aliased) > 0) {
      aliased <- paste0(
        parts$input_columns$aliased$from,
        " \u2192 ",
        parts$input_columns$aliased$to
      )
      output <- c(
        output,
        paste0(
          "<strong>Aliased Columns:</strong> ",
          paste(aliased, collapse = ", ")
        ),
        ""
      )
    }
  }

  if (!is.null(parts$tables$theta)) {
    output <- c(
      output,
      "",
      print_data_table_knit(parts$tables$theta$data, parts$tables$theta$title)
    )
  }
  if (!is.null(parts$tables$omega)) {
    output <- c(
      output,
      "",
      print_data_table_knit(parts$tables$omega$data, parts$tables$omega$title)
    )
  }
  if (!is.null(parts$tables$sigma)) {
    output <- c(
      output,
      "",
      print_data_table_knit(parts$tables$sigma$data, parts$tables$sigma$title)
    )
  }

  # Return as HTML
  knitr::asis_output(paste(output, collapse = "\n"))
}

#' Get model name
#'
#' Extracts the model name from a hyperion_nonmem_model object.
#'
#' @param model A hyperion_nonmem_model object
#' @return Character string with the model name (filename without extension)
#' @export
#'
#' @examples
#' \dontrun{
#' mod <- read_model("models/run001.mod")
#' get_model_name(mod) # "run001"
#' }
get_model_name <- function(model) {
  model_source <- attr(model, "model_source")
  if (is.null(model_source)) {
    rlang::abort("Model object is missing model_source attribute")
  }
  tools::file_path_sans_ext(basename(model_source))
}

#' Get model directory
#'
#' Extracts the directory path from a hyperion_nonmem_model object.
#'
#' @param model A hyperion_nonmem_model object
#' @param absolute Logical; if TRUE, resolve `model_source` using
#'   [from_config_relative()] before extracting the directory. Default FALSE.
#' @return Character string with the model directory path
#' @export
#'
#' @examples
#' \dontrun{
#' mod <- read_model("models/run001.mod")
#' get_model_dir(mod) # "models" (config-relative)
#' get_model_dir(mod, absolute = TRUE) # "/path/to/project/models"
#' }
get_model_dir <- function(model, absolute = FALSE) {
  model_source <- attr(model, "model_source")
  if (is.null(model_source)) {
    rlang::abort("Model object is missing model_source attribute")
  }

  if (!is.logical(absolute) || length(absolute) != 1 || is.na(absolute)) {
    rlang::abort("absolute must be TRUE or FALSE")
  }

  if (absolute) {
    model_source <- from_config_relative(model_source)
  }

  dirname(model_source)
}

#' Get data path
#'
#' Extracts the dataset path from a hyperion_nonmem_model object.
#'
#' @param model A hyperion_nonmem_model object
#' @return Character string with the data path, or NULL if not found
#' @export
#'
#' @examples
#' \dontrun{
#' mod <- read_model("models/run001.mod")
#' get_data_path(mod) # "../data/derived/pk_data.csv"
#' }
get_data_path <- function(model) {
  model$data$path
}
