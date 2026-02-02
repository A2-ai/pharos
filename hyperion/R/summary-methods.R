#' Load summary configuration thresholds from pharos.toml
#'
#' @return Named list with correlation_threshold and condition_threshold
#' @keywords internal
#' @noRd
load_summary_config_thresholds <- function() {
  tryCatch(
    {
      config <- get_pharos_config()

      list(
        correlation_threshold = config$nonmem$summary$high_correlation_threshold,
        condition_threshold = config$nonmem$summary$high_condition_threshold
      )
    },
    error = function(e) {
      warning(
        "pharos.toml file could not be found. Using defaults (correlation_threshold = 0.95, condition_threshold = 1000).",
        call. = FALSE
      )
      list(
        correlation_threshold = 0.95,
        condition_threshold = 1000
      )
    }
  )
}

#' Process heuristics data into ordered messages
#'
#' @param run_heuristics Data frame with heuristic results
#' @return Data frame with ordered heuristics and positive/negative messages
#' @keywords internal
#' @noRd
process_heuristics_data <- function(run_heuristics) {
  if (nrow(run_heuristics) == 0) {
    return(data.frame())
  }

  # Define the order and messages
  heuristic_order <- c(
    "minimization_terminated",
    "covariance_step_aborted",
    "eigenvalue_issues",
    "parameter_near_boundary",
    "hessian_reset"
  )

  positive_messages <- list(
    "minimization_terminated" = "Minimization Successful",
    "covariance_step_aborted" = "Covariance Step Successful",
    "eigenvalue_issues" = "No Eigenvalue Issues",
    "parameter_near_boundary" = "No Parameters Near Boundary",
    "hessian_reset" = "No Hessian Resets"
  )

  negative_messages <- list(
    "minimization_terminated" = "Minimization Terminated",
    "covariance_step_aborted" = "Covariance Step Aborted",
    "eigenvalue_issues" = "Eigenvalue Issues Detected",
    "parameter_near_boundary" = "Parameters Near Boundary",
    "hessian_reset" = "Hessian Reset Occurred"
  )

  # Build ordered results
  results <- data.frame()
  for (heuristic_name in heuristic_order) {
    if (heuristic_name %in% run_heuristics$heuristic_name) {
      has_issue <- run_heuristics$value[
        run_heuristics$heuristic_name == heuristic_name
      ]

      message <- if (has_issue) {
        negative_messages[[heuristic_name]]
      } else {
        positive_messages[[heuristic_name]]
      }

      results <- rbind(
        results,
        data.frame(
          heuristic = heuristic_name,
          has_issue = has_issue,
          message = message,
          stringsAsFactors = FALSE
        )
      )
    }
  }

  return(results)
}

#' Filter and sort high correlations
#'
#' @param correlation_matrix Full correlation matrix data frame
#' @param threshold Correlation threshold for filtering
#' @return Filtered and sorted correlation data frame with method extracted
#' @keywords internal
#' @noRd
filter_and_sort_correlations <- function(correlation_matrix, threshold) {
  if (is.null(correlation_matrix) || nrow(correlation_matrix) == 0) {
    return(list(correlations = data.frame(), method = NULL))
  }

  # Filter high correlations
  high_corr <- correlation_matrix[
    abs(correlation_matrix$correlation) >= threshold,
  ]

  if (nrow(high_corr) == 0) {
    return(list(correlations = data.frame(), method = NULL))
  }

  # Sort by absolute correlation value (highest first)
  high_corr <- high_corr[order(abs(high_corr$correlation), decreasing = TRUE), ]

  # Get method from first row (assuming all are the same)
  method <- high_corr$method[1]

  list(
    correlations = high_corr,
    method = method
  )
}

#' @noRd
build_summary_display_parts <- function(x, digits = NULL) {
  run_name <- x$run_name
  run_details <- x$run_details
  run_heuristics <- x$run_heuristics
  minimization_results <- x$minimization_results
  parameters <- x$parameters
  correlation_matrix <- x$correlation_matrix

  thresholds <- load_summary_config_thresholds()
  correlation_threshold <- thresholds$correlation_threshold
  condition_threshold <- thresholds$condition_threshold

  title <- if (!is.null(run_name)) {
    paste0("Model Summary: ", run_name)
  } else {
    "Model Summary"
  }

  problem <- NULL
  records_line <- NULL
  if (nrow(run_details) > 0) {
    problem <- run_details$problem[1]
    records_line <- paste0(
      "Records: ",
      run_details$number_data_records[1],
      " | Observations: ",
      run_details$number_obs[1],
      " | Subjects: ",
      run_details$number_subjects[1]
    )
  }

  ofv_display <- NULL
  if (nrow(minimization_results) > 0) {
    ofv_values <- minimization_results$ofv[!is.na(minimization_results$ofv)]
    if (length(ofv_values) > 0) {
      ofv_display <- format_hyperion_sigfig_string(
        utils::tail(ofv_values, 1),
        digits
      )
    }
  }

  estimation_methods <- list()
  if (nrow(run_details) > 0) {
    estimation_methods <- lapply(seq_len(nrow(run_details)), function(i) {
      cond_num <- NA_real_
      term_status <- NA_character_
      if (nrow(minimization_results) >= i) {
        cond_num <- minimization_results$condition_number[i]
        term_status <- minimization_results$termination_status[i]
      }

      list(
        method = run_details$estimation_method[i],
        cond_num = cond_num,
        cond_num_display = format_hyperion_sigfig_string(cond_num, digits),
        cond_num_is_high = !is.na(cond_num) && cond_num > condition_threshold,
        term_status = term_status
      )
    })
  }

  heuristic_results <- process_heuristics_data(run_heuristics)

  corr_result <- filter_and_sort_correlations(
    correlation_matrix,
    correlation_threshold
  )
  correlations <- NULL
  if (nrow(corr_result$correlations) > 0) {
    corr_display_df <- data.frame(
      `Parameter 1` = corr_result$correlations$param1,
      `Parameter 2` = corr_result$correlations$param2,
      Correlation = corr_result$correlations$correlation,
      stringsAsFactors = FALSE,
      check.names = FALSE
    )
    summary_line <- paste0(
      "Threshold: ",
      correlation_threshold,
      ", Method: ",
      corr_result$method
    )
    correlations <- list(
      title = paste0(
        "High Correlations (threshold: ",
        correlation_threshold,
        ", method: ",
        corr_result$method,
        ")"
      ),
      summary_line = summary_line,
      table = format_display_data(corr_display_df, digits)
    )
  }

  parameter_tables <- list()
  if (nrow(parameters) > 0) {
    kinds <- unique(parameters$kind)
    parameter_tables <- lapply(kinds, function(kind) {
      subset_params <- parameters[parameters$kind == kind, ]
      list(
        title = tools::toTitleCase(paste(tolower(kind), "Parameters")),
        table = format_display_data(subset_params, digits)
      )
    })
  }

  list(
    title = title,
    problem = problem,
    records_line = records_line,
    ofv_display = ofv_display,
    estimation_methods = estimation_methods,
    heuristic_results = heuristic_results,
    correlations = correlations,
    parameter_tables = parameter_tables
  )
}

#' Print running model summary
#'
#' @param x A hyperion_nonmem_summary object for a running model
#' @param digits Number of significant digits
#' @return Invisible copy of x
#' @keywords internal
#' @noRd
print_running_summary <- function(x, digits = NULL) {
  title <- if (!is.null(x$run_name)) {
    paste0("Running Model: ", x$run_name)
  } else {
    "Running Model Summary"
  }

  cli::cli_text("")
  cli::cli_h1(title)
  cli::cli_alert_info("Model is currently running")

  if (!is.null(x$iterations) && nrow(x$iterations) > 0) {
    cli::cli_h2("Recent Iterations")
    formatted <- format_display_data(x$iterations, digits)
    print_data_table_console(formatted, NULL)
  } else {
    cli::cli_alert_warning("No iteration data available yet")
  }

  if (!is.null(x$gradients) && nrow(x$gradients) > 0) {
    cli::cli_h2("Recent Gradients")
    formatted <- format_display_data(x$gradients, digits)
    print_data_table_console(formatted, NULL)
  }

  invisible(x)
}

#' Print method for hyperion_nonmem_summary objects
#'
#' @param x A hyperion_nonmem_summary object (list with run_name, run_details, run_heuristics, minimization_results, parameters)
#' @param digits Number of significant digits (uses global option if NULL)
#' @param ... Additional arguments (ignored)
#' @return Invisible copy of x
#' @rawNamespace S3method(base::print, hyperion_nonmem_summary)
print.hyperion_nonmem_summary <- function(x, digits = NULL, ...) {
  # Check if this is a running summary (has iterations field)
  if (!is.null(x$iterations)) {
    print_running_summary(x, digits)
    return(invisible(x))
  }

  parts <- build_summary_display_parts(x, digits)

  cli::cli_text("")
  cli::cli_h1(parts$title)

  if (!is.null(parts$problem)) {
    cli::cli_text("{.strong Problem:} {parts$problem}")
  }
  if (!is.null(parts$records_line)) {
    cli::cli_text("{.strong {parts$records_line}}")
  }

  if (!is.null(parts$ofv_display)) {
    cli::cli_text("{.strong Final OFV:} {parts$ofv_display}")
  }

  if (length(parts$estimation_methods) > 0) {
    cli::cli_h2("Estimation Methods")
    cli::cli_ul()
    for (i in seq_along(parts$estimation_methods)) {
      method <- parts$estimation_methods[[i]]
      cli::cli_li("{method$method}")

      if (!is.na(method$cond_num)) {
        cond_num_display <- if (method$cond_num_is_high) {
          cli::col_red(method$cond_num_display)
        } else {
          method$cond_num_display
        }

        if (!is.na(method$term_status)) {
          cli::cli_bullets(c(
            " " = "Condition Number: {cond_num_display}, Termination Status: {method$term_status}"
          ))
        } else {
          cli::cli_bullets(c(" " = "Condition Number: {cond_num_display}"))
        }
      }

      if (i < length(parts$estimation_methods)) {
        cli::cli_text("")
      }
    }
    cli::cli_end()
  }

  cli::cli_h2("Heuristic Checks")
  if (nrow(parts$heuristic_results) > 0) {
    for (i in seq_len(nrow(parts$heuristic_results))) {
      result <- parts$heuristic_results[i, ]
      if (result$has_issue) {
        cli::cli_text("[{cli::col_red(cli::symbol$cross)}] {result$message}")
      } else {
        cli::cli_text("[{cli::col_green('OK')}] {result$message}")
      }
    }
  } else {
    cli::cli_alert_info("No heuristic checks available")
  }

  if (!is.null(parts$correlations)) {
    print_data_table_console(
      parts$correlations$table,
      parts$correlations$title
    )
  }

  if (length(parts$parameter_tables) > 0) {
    for (table in parts$parameter_tables) {
      print_data_table_console(table$table, table$title)
    }
  }

  invisible(x)
}

#' Knit print running model summary (for Quarto/R Markdown)
#' @param x A hyperion_nonmem_summary object for a running model
#' @return HTML/markdown output for rendered documents
#' @keywords internal
#' @noRd
knit_print_running_summary <- function(x) {
  title <- if (!is.null(x$run_name)) {
    paste0("Running Model: ", x$run_name)
  } else {
    "Running Model Summary"
  }

  output <- character()
  output <- c(output, "", paste0("<strong>", title, "</strong>"), "")
  output <- c(
    output,
    '<p style="color:#0066cc">Model is currently running</p>',
    ""
  )

  if (!is.null(x$iterations) && nrow(x$iterations) > 0) {
    output <- c(output, "", "<strong>Recent Iterations</strong>", "")
    formatted <- format_display_data(x$iterations, NULL)
    output <- c(output, print_data_table_knit(formatted, NULL))
  } else {
    output <- c(output, "<p>No iteration data available yet</p>", "")
  }

  if (!is.null(x$gradients) && nrow(x$gradients) > 0) {
    output <- c(output, "", "<strong>Recent Gradients</strong>", "")
    formatted <- format_display_data(x$gradients, NULL)
    output <- c(output, print_data_table_knit(formatted, NULL))
  }

  knitr::asis_output(paste(output, collapse = "\n"))
}

#' Knit print method for hyperion_nonmem_summary objects (for Quarto/R Markdown)
#' @param x A hyperion_nonmem_summary object
#' @param ... Additional arguments (ignored)
#' @return HTML/markdown output for rendered documents
#' @exportS3Method knitr::knit_print
knit_print.hyperion_nonmem_summary <- function(x, ...) {
  # Check if this is a running summary
  if (!is.null(x$iterations)) {
    return(knit_print_running_summary(x))
  }

  parts <- build_summary_display_parts(x)
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
  if (!is.null(parts$records_line)) {
    output <- c(
      output,
      paste0("<strong>", parts$records_line, "</strong>"),
      ""
    )
  }

  if (!is.null(parts$ofv_display)) {
    output <- c(
      output,
      paste0("<strong>Final OFV:</strong> ", parts$ofv_display),
      ""
    )
  }

  if (length(parts$estimation_methods) > 0) {
    output <- c(
      output,
      "",
      '<strong>Estimation Methods</strong>',
      ""
    )

    for (method in parts$estimation_methods) {
      output <- c(output, paste0("- <strong>", method$method, "</strong>"))

      if (!is.na(method$cond_num)) {
        cond_num_display <- if (method$cond_num_is_high) {
          paste0('<span style="color:red">', method$cond_num_display, "</span>")
        } else {
          method$cond_num_display
        }

        if (!is.na(method$term_status)) {
          output <- c(
            output,
            paste0(
              "  - Condition Number: ",
              cond_num_display,
              ", Termination Status: ",
              method$term_status
            )
          )
        } else {
          output <- c(
            output,
            paste0("  - Condition Number: ", cond_num_display)
          )
        }
      }
      output <- c(output, "")
    }
  }

  # Heuristics
  output <- c(
    output,
    "",
    '<strong>Heuristic Checks</strong>',
    ""
  )

  if (nrow(parts$heuristic_results) > 0) {
    for (i in seq_len(nrow(parts$heuristic_results))) {
      result <- parts$heuristic_results[i, ]
      if (result$has_issue) {
        output <- c(
          output,
          paste0('[<span style="color:red">\u2716</span>] ', result$message),
          ""
        )
      } else {
        output <- c(
          output,
          paste0('[<span style="color:green">OK</span>] ', result$message),
          ""
        )
      }
    }
  } else {
    output <- c(output, "No heuristic checks available", "")
  }

  if (!is.null(parts$correlations)) {
    output <- c(
      output,
      "",
      paste0("<strong>", parts$correlations$summary_line, "</strong>"),
      ""
    )
    output <- c(
      output,
      "",
      print_data_table_knit(parts$correlations$table, "High Correlations")
    )
  }

  if (length(parts$parameter_tables) > 0) {
    for (table in parts$parameter_tables) {
      output <- c(
        output,
        "",
        print_data_table_knit(table$table, table$title)
      )
    }
  }

  # Return as HTML
  knitr::asis_output(paste(output, collapse = "\n"))
}
