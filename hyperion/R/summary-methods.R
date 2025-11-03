#' Load summary configuration thresholds from pharos.toml
#'
#' @return Named list with correlation_threshold and condition_threshold
#' @keywords internal
#' @noRd
load_summary_config_thresholds <- function() {
  config <- tryCatch(
    {
      get_pharos_config()
    },
    error = function(e) {
      list(
        nonmem = list(
          summary = list(
            high_correlation_threshold = 0.95,
            high_condition_threshold = 1000
          )
        )
      )
    }
  )

  correlation_threshold <- config$nonmem$summary$high_correlation_threshold
  if (is.null(correlation_threshold)) correlation_threshold <- 0.95

  condition_threshold <- config$nonmem$summary$high_condition_threshold
  if (is.null(condition_threshold)) condition_threshold <- 1000

  list(
    correlation_threshold = correlation_threshold,
    condition_threshold = condition_threshold
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

#' Print method for hyperion_nonmem_summary objects
#'
#' @param x A hyperion_nonmem_summary object (list with run_name, run_details, run_heuristics, minimization_results, parameters)
#' @param digits Number of significant digits (uses global option if NULL)
#' @param ... Additional arguments (ignored)
#' @return Invisible copy of x
#' @export
print.hyperion_nonmem_summary <- function(x, digits = NULL, ...) {
  # Extract data
  run_name <- x$run_name
  run_details <- x$run_details
  run_heuristics <- x$run_heuristics
  minimization_results <- x$minimization_results
  parameters <- x$parameters
  correlation_matrix <- x$correlation_matrix

  # Get config thresholds
  thresholds <- load_summary_config_thresholds()
  correlation_threshold <- thresholds$correlation_threshold
  condition_threshold <- thresholds$condition_threshold

  # Header with run name
  if (!is.null(run_name)) {
    cli::cli_h1("Model Summary: {run_name}")
  } else {
    cli::cli_h1("Model Summary")
  }

  # Problem info (from first row of run_details)
  if (nrow(run_details) > 0) {
    cli::cli_text("{.strong Problem:} {run_details$problem[1]}")
    cli::cli_text(
      "{.strong Records:} {run_details$number_data_records[1]} | ",
      "{.strong Observations:} {run_details$number_obs[1]} | ",
      "{.strong Subjects:} {run_details$number_subjects[1]}"
    )
  }

  if (nrow(minimization_results) > 0) {
    # OFV info if available
    ofv_values <- minimization_results$ofv[!is.na(minimization_results$ofv)]
    if (length(ofv_values) > 0) {
      cli::cli_text(
        "{.strong Final OFV:} {.val {format_hyperion_number(utils::tail(ofv_values, 1))}}"
      )
    }
  }

  # Estimation methods with details
  if (nrow(run_details) > 0) {
    cli::cli_h2("Estimation Methods")

    for (i in seq_len(nrow(run_details))) {
      method <- run_details$estimation_method[i]
      cli::cli_ul()
      cli::cli_li("{method}")

      # Get condition number and termination status from minimization_results
      if (nrow(minimization_results) >= i) {
        cond_num <- format_hyperion_number(minimization_results$condition_number[
          i
        ])
        term_status <- minimization_results$termination_status[i]

        # Color condition number red if > threshold
        cond_num_display <- if (
          !is.na(cond_num) && cond_num > condition_threshold
        ) {
          cli::col_red(cond_num)
        } else {
          cond_num
        }

        if (!is.na(term_status)) {
          cli::cli_bullets(c(
            " " = "Condition Number: {cond_num_display}, Termination Status: {term_status}"
          ))
        } else {
          cli::cli_bullets(c(" " = "Condition Number: {cond_num_display}"))
        }
      }

      # Add blank line between methods
      if (i < nrow(run_details)) {
        cli::cli_text("")
      }
    }
  }
  # Heuristics - show all checks with pass/fail status
  cli::cli_h2("Heuristic Checks")
  heuristic_results <- process_heuristics_data(run_heuristics)

  if (nrow(heuristic_results) > 0) {
    for (i in seq_len(nrow(heuristic_results))) {
      result <- heuristic_results[i, ]
      if (result$has_issue) {
        cli::cli_text("[{cli::col_red(cli::symbol$cross)}] {result$message}")
      } else {
        cli::cli_text("[{cli::col_green('OK')}] {result$message}")
      }
    }
  } else {
    cli::cli_alert_info("No heuristic checks available")
  }

  # High correlations section
  corr_result <- filter_and_sort_correlations(
    correlation_matrix,
    correlation_threshold
  )
  if (nrow(corr_result$correlations) > 0) {
    # Build display table (without Method column)
    corr_display_df <- data.frame(
      `Parameter 1` = corr_result$correlations$param1,
      `Parameter 2` = corr_result$correlations$param2,
      Correlation = corr_result$correlations$correlation,
      stringsAsFactors = FALSE,
      check.names = FALSE
    )

    # Format numbers and print
    formatted_corr <- format_display_data(corr_display_df, digits)
    title <- paste0(
      "High Correlations (threshold: ",
      correlation_threshold,
      ", method: ",
      corr_result$method,
      ")"
    )
    print_data_table_console(formatted_corr, title)
  }

  # Parameter tables
  if (nrow(parameters) > 0) {
    if ("kind" %in% names(parameters)) {
      # Group by kind if available
      kinds <- unique(parameters$kind)
      for (kind in kinds) {
        subset_params <- parameters[parameters$kind == kind, ]
        formatted_params <- format_display_data(subset_params, digits)
        title <- tools::toTitleCase(paste(tolower(kind), "Parameters"))
        print_data_table_console(formatted_params, title)
      }
    } else {
      # Try to infer parameter types from names, or print unified table
      theta_params <- parameters[grepl("^THETA", parameters$name), ]
      omega_params <- parameters[grepl("^(OMEGA\\(|ETA)", parameters$name), ]
      sigma_params <- parameters[grepl("^(SIGMA\\(|EPS)", parameters$name), ]

      if (nrow(theta_params) > 0) {
        formatted_theta <- format_display_data(theta_params, digits)
        print_data_table_console(formatted_theta, "Theta Parameters")
      }
      if (nrow(omega_params) > 0) {
        formatted_omega <- format_display_data(omega_params, digits)
        print_data_table_console(formatted_omega, "Omega Parameters")
      }
      if (nrow(sigma_params) > 0) {
        formatted_sigma <- format_display_data(sigma_params, digits)
        print_data_table_console(formatted_sigma, "Sigma Parameters")
      }

      # Handle any remaining parameters that don't match the patterns
      other_params <- parameters[
        !grepl("^(THETA|OMEGA\\(|ETA|SIGMA\\(|EPS)", parameters$name),
      ]
      if (nrow(other_params) > 0) {
        formatted_other <- format_display_data(other_params, digits)
        print_data_table_console(formatted_other, "Other Parameters")
      }
    }
  }

  invisible(x)
}

#' Knit print method for hyperion_nonmem_summary objects (for Quarto/R Markdown)
#' @param x A hyperion_nonmem_summary object
#' @param ... Additional arguments (ignored)
#' @return HTML/markdown output for rendered documents
#' @exportS3Method knitr::knit_print
knit_print.hyperion_nonmem_summary <- function(x, ...) {
  # Extract data
  run_name <- x$run_name
  run_details <- x$run_details
  run_heuristics <- x$run_heuristics
  minimization_results <- x$minimization_results
  parameters <- x$parameters
  correlation_matrix <- x$correlation_matrix

  # Get config thresholds
  thresholds <- load_summary_config_thresholds()
  correlation_threshold <- thresholds$correlation_threshold
  condition_threshold <- thresholds$condition_threshold

  # Build markdown output
  output <- character()

  # Header
  if (!is.null(run_name)) {
    output <- c(output, paste0("# Model Summary: ", run_name), "")
  } else {
    output <- c(output, "# Model Summary", "")
  }

  # Problem info
  if (nrow(run_details) > 0) {
    output <- c(output, paste0("**Problem:** ", run_details$problem[1]), "")
    output <- c(
      output,
      paste0(
        "**Records:** ",
        run_details$number_data_records[1],
        " | **Observations:** ",
        run_details$number_obs[1],
        " | **Subjects:** ",
        run_details$number_subjects[1]
      ),
      ""
    )
  }

  # OFV info
  if (nrow(minimization_results) > 0) {
    ofv_values <- minimization_results$ofv[!is.na(minimization_results$ofv)]
    if (length(ofv_values) > 0) {
      output <- c(
        output,
        paste0(
          "**Final OFV:** ",
          format_hyperion_number(utils::tail(ofv_values, 1))
        ),
        ""
      )
    }
  }

  # Estimation methods
  if (nrow(run_details) > 0) {
    output <- c(output, "## Estimation Methods", "")

    for (i in seq_len(nrow(run_details))) {
      method <- run_details$estimation_method[i]
      output <- c(output, paste0("- **", method, "**"))

      if (nrow(minimization_results) >= i) {
        cond_num <- format_hyperion_number(minimization_results$condition_number[
          i
        ])
        term_status <- minimization_results$termination_status[i]

        # Color condition number red if > threshold
        cond_num_display <- if (
          !is.na(cond_num) && cond_num > condition_threshold
        ) {
          paste0('<span style="color:red">', cond_num, '</span>')
        } else {
          cond_num
        }

        if (!is.na(term_status)) {
          output <- c(
            output,
            paste0(
              "  - Condition Number: ",
              cond_num_display,
              ", Termination Status: ",
              term_status
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
  output <- c(output, "## Heuristic Checks", "")
  heuristic_results <- process_heuristics_data(run_heuristics)

  if (nrow(heuristic_results) > 0) {
    for (i in seq_len(nrow(heuristic_results))) {
      result <- heuristic_results[i, ]
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

  # High correlations section
  corr_result <- filter_and_sort_correlations(
    correlation_matrix,
    correlation_threshold
  )
  if (nrow(corr_result$correlations) > 0) {
    output <- c(
      output,
      "",
      paste0(
        "**Threshold:** ",
        correlation_threshold,
        ", **Method:** ",
        corr_result$method
      ),
      ""
    )

    # Build display table (without Method column)
    corr_display_df <- data.frame(
      `Parameter 1` = corr_result$correlations$param1,
      `Parameter 2` = corr_result$correlations$param2,
      Correlation = corr_result$correlations$correlation,
      stringsAsFactors = FALSE,
      check.names = FALSE
    )

    # Format and create table output using unified approach
    formatted_corr <- format_display_data(corr_display_df)
    output <- c(
      output,
      print_data_table_knit(formatted_corr, "High Correlations")
    )
  }

  # Parameter tables using kable
  if (nrow(parameters) > 0) {
    if ("kind" %in% names(parameters)) {
      kinds <- unique(parameters$kind)
      for (kind in kinds) {
        subset_params <- parameters[parameters$kind == kind, ]
        formatted_params <- format_display_data(subset_params)
        title <- tools::toTitleCase(paste(tolower(kind), "Parameters"))
        output <- c(output, print_data_table_knit(formatted_params, title))
      }
    } else {
      # Fallback logic for when kind column is not present
      theta_params <- parameters[grepl("^THETA", parameters$name), ]
      omega_params <- parameters[grepl("^(OMEGA\\(|ETA)", parameters$name), ]
      sigma_params <- parameters[grepl("^(SIGMA\\(|EPS)", parameters$name), ]

      if (nrow(theta_params) > 0) {
        formatted_theta <- format_display_data(theta_params)
        output <- c(
          output,
          print_data_table_knit(formatted_theta, "Theta Parameters")
        )
      }
      if (nrow(omega_params) > 0) {
        formatted_omega <- format_display_data(omega_params)
        output <- c(
          output,
          print_data_table_knit(formatted_omega, "Omega Parameters")
        )
      }
      if (nrow(sigma_params) > 0) {
        formatted_sigma <- format_display_data(sigma_params)
        output <- c(
          output,
          print_data_table_knit(formatted_sigma, "Sigma Parameters")
        )
      }
    }
  }

  # Return as HTML
  knitr::asis_output(paste(output, collapse = "\n"))
}
