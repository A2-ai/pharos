#' Audit parameter information sources
#'
#' Shows where each piece of parameter metadata originated (mod file path,
#' lookup file path, "default", or "hard-coded").
#'
#' @param info ModelComments object from get_model_parameter_info()
#' @return List with theta, omega, sigma data frames showing source paths for each field
#' @export
audit_parameter_info <- function(info) {
  if (!S7::S7_inherits(info, ModelComments)) {
    stop("info must be a ModelComments object from get_model_parameter_info()")
  }

  result <- list(
    theta = audit_comment_list(
      info@theta,
      theta_fields()
    ),
    omega = audit_comment_list(
      info@omega,
      omega_fields()
    ),
    sigma = audit_comment_list(
      info@sigma,
      sigma_fields()
    )
  )
  class(result) <- "parameter_audit"
  result
}

#' @noRd
audit_comment_list <- function(comments, fields) {
  if (length(comments) == 0) {
    # Return empty data frame with correct columns
    df <- data.frame(parameter = character(), stringsAsFactors = FALSE)
    for (f in fields) df[[f]] <- character()
    return(df)
  }

  rows <- lapply(names(comments), function(nm) {
    cmt <- comments[[nm]]
    sources <- attr(cmt, "sources") %||% list()

    row <- data.frame(parameter = nm, stringsAsFactors = FALSE)
    for (f in fields) {
      row[[f]] <- sources[[f]] %||% NA_character_
    }
    row
  })
  do.call(rbind, rows)
}

#' print method for parameter_audit objects
#' @param x A parameter_audit object
#' @param ... Additional arguments (ignored)
#' @return Invisible copy of x
#' @rawNamespace S3method(base::print, parameter_audit)
print.parameter_audit <- function(x, ...) {
  cli::cli_h1("Parameter Info Audit")

  if (nrow(x$theta) > 0) {
    print_data_table_console(x$theta, "Theta Sources")
  }
  if (nrow(x$omega) > 0) {
    print_data_table_console(x$omega, "Omega Sources")
  }
  if (nrow(x$sigma) > 0) {
    print_data_table_console(x$sigma, "Sigma Sources")
  }
  invisible(x)
}

#' knit_print method for parameter_audit objects
#' @param x A parameter_audit object
#' @param ... Additional arguments (ignored)
#' @return HTML/markdown output for rendered documents
#' @exportS3Method knitr::knit_print
knit_print.parameter_audit <- function(x, ...) {
  output <- character()
  output <- c(output, "# Parameter Info Audit", "")

  if (nrow(x$theta) > 0) {
    output <- c(output, print_data_table_knit(x$theta, "Theta Sources"))
  }
  if (nrow(x$omega) > 0) {
    output <- c(output, print_data_table_knit(x$omega, "Omega Sources"))
  }
  if (nrow(x$sigma) > 0) {
    output <- c(output, print_data_table_knit(x$sigma, "Sigma Sources"))
  }

  knitr::asis_output(paste(output, collapse = "\n"))
}
