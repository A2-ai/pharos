#' Print method for hyperion_summary objects
#'
#' @param x A hyperion_summary object (list with run_name, run_details, run_heuristics, minimization_results, parameters)
#' @param ... Additional arguments (ignored)
#' @return Invisible copy of x
#' @export
print.hyperion_summary <- function(x, ...) {
  # Extract data
  run_name <- x$run_name
  run_details <- x$run_details
  run_heuristics <- x$run_heuristics
  minimization_results <- x$minimization_results
  parameters <- x$parameters

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
      cli::cli_text("{.strong Final OFV:} {.val {round(tail(ofv_values, 1), 3)}}")
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
        cond_num <- round(minimization_results$condition_number[i], 1)
        term_status <- minimization_results$termination_status[i]

        if (!is.na(term_status)) {
          cli::cli_bullets(c(" " = "Condition Number: {cond_num}, Termination Status: {term_status}"))
        } else {
          cli::cli_bullets(c(" " = "Condition Number: {cond_num}"))
        }
      }

      # Add blank line between methods
      if (i < nrow(run_details)) {
        cli::cli_text("")
      }
    }
  }
  # Heuristics
  problems <- run_heuristics$heuristic_name[run_heuristics$value == TRUE]
  cli::cli_h2("Heuristic Problems")
  if (length(problems) == 0) {
    cli::cli_alert_success("None detected")
  } else {
    problem_names <- sapply(problems, function(p) {
      # Make names more readable
      readable_name <- gsub("_", " ", p)
      tools::toTitleCase(readable_name)
    })
    cli::cli_alert_warning("Issues detected:")
    cli::cli_ul(problem_names)
  }

  # Parameter tables
  if (nrow(parameters) > 0) {
    if ("kind" %in% names(parameters)) {
      # Group by kind if available
      kinds <- unique(parameters$kind)
      for (kind in kinds) {
        subset_params <- parameters[parameters$kind == kind, ]
        print_parameter_table_cli(subset_params, kind)
      }
    } else {
      # Try to infer parameter types from names, or print unified table
      theta_params <- parameters[grepl("^THETA", parameters$name), ]
      omega_params <- parameters[grepl("^(OMEGA\\(|ETA)", parameters$name), ]
      sigma_params <- parameters[grepl("^(SIGMA\\(|EPS)", parameters$name), ]

      if (nrow(theta_params) > 0) {
        print_parameter_table_cli(theta_params, "Theta")
      }
      if (nrow(omega_params) > 0) {
        print_parameter_table_cli(omega_params, "Omega")
      }
      if (nrow(sigma_params) > 0) {
        print_parameter_table_cli(sigma_params, "Sigma")
      }

      # Handle any remaining parameters that don't match the patterns
      other_params <- parameters[!grepl("^(THETA|OMEGA\\(|ETA|SIGMA\\(|EPS)", parameters$name), ]
      if (nrow(other_params) > 0) {
        print_parameter_table_cli(other_params, "Other")
      }
    }
  }

  invisible(x)
}

#' Helper function to print parameter tables using cli
#' @param params Parameter dataframe subset
#' @param kind Parameter type (THETA, OMEGA, SIGMA)
#' @keywords internal
#' @noRd
print_parameter_table_cli <- function(params, kind) {
  if (nrow(params) == 0) {
    return()
  }

  cli::cat_line(" ")
  heading <- tools::toTitleCase(paste(tolower(kind), "Parameters"))
  cli::cli_h2(heading)

  # Check what columns are available
  has_stderr <- "stderr" %in% names(params)
  has_rse <- "rse" %in% names(params)
  has_shrinkage <- "shrinkage" %in% names(params)
  has_fixed <- "fixed" %in% names(params)

  # Build the display table
  display_df <- data.frame(
    Parameter = params$name,
    Estimate = round(params$value, 4),
    stringsAsFactors = FALSE
  )

  # Add separate SE and RSE columns if available
  if (has_stderr) {
    display_df$SE <- round(params$stderr, 4)
  }
  if (has_rse) {
    display_df$`RSE (%)` <- round(params$rse, 2)
  }

  # Add shrinkage for Omega if available
  if (kind %in% c("OMEGA", "Omega", "Sigma", "SIGMA") && has_shrinkage) {
    display_df$`Shrinkage (%)` <- ifelse(is.na(params$shrinkage), NA_real_,
      sprintf("%.2f", params$shrinkage)
    )
  }

  # Add fixed status if available
  if (has_fixed) {
    display_df$Fixed <- ifelse(params$fixed, "yes", "no")
  }

  # Print using cli table formatting for console output
  if (requireNamespace("cli", quietly = TRUE)) {
    # Format numeric columns for better display
    for (col in names(display_df)) {
      if (col == "Estimate" || grepl("Shrinkage", col)) {
        display_df[[col]] <- sprintf("%.4f", as.numeric(display_df[[col]]))
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

    cli::cat_line(" ")
    cli::cat_line(paste(header_parts, collapse = "  "))
    cli::cat_line(paste(sapply(col_widths, function(w) paste(rep("─", w), collapse = "")), collapse = "  "))

    # Print rows with proper alignment - pad first, then style
    for (i in seq_len(nrow(display_df))) {
      row_parts <- sapply(seq_len(ncol(display_df)), function(j) {
        cell_data <- as.character(display_df[i, j])
        col_name <- names(display_df)[j]

        # Apply padding first (using plain text)
        padded_cell <- sprintf("%-*s", col_widths[j], cell_data)

        # Apply styling after padding based on column and content
        if (col_name == "Parameter" && grepl("^(THETA|ETA|EPS)", cell_data)) {
          # All parameter names in blue
          padded_cell <- cli::col_blue(padded_cell)
        } else if (col_name == "Estimate" && grepl("^[0-9]", cell_data)) {
          # Estimates in green
          padded_cell <- cli::col_green(padded_cell)
        } else if (col_name == "RSE (%)" && !is.na(suppressWarnings(as.numeric(cell_data))) && suppressWarnings(as.numeric(cell_data)) > 30) {
          # RSE% > 30% in red
          padded_cell <- cli::col_red(padded_cell)
        }

        return(padded_cell)
      })

      cli::cat_line(paste(row_parts, collapse = "  "))
    }
  } else if (requireNamespace("knitr", quietly = TRUE)) {
    # Enhanced knitr::kable with better formatting
    formatted_table <- knitr::kable(display_df,
      format = "simple",
      digits = 4,
      align = c("l", rep("r", ncol(display_df) - 1))
    )
    cat(formatted_table, sep = "\n")
    cat("\n")
  } else {
    # Final fallback to base print with better formatting
    print(display_df, row.names = FALSE, digits = 4)
    cat("\n")
  }
}
