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
    theta = NULL,
    omega = NULL,
    sigma = NULL
  )

  tables <- build_comment_tables(
    list(theta = info@theta, omega = info@omega, sigma = info@sigma),
    list(
      theta = theta_fields(),
      omega = omega_fields(),
      sigma = sigma_fields()
    ),
    function(cmt, field) {
      sources <- attr(cmt, "sources") %||% list()
      sources[[field]]
    }
  )

  result$theta <- tables$theta
  result$omega <- tables$omega
  result$sigma <- tables$sigma

  class(result) <- "parameter_audit"
  result
}

#' print method for parameter_audit objects
#' @param x A parameter_audit object
#' @param ... Additional arguments (ignored)
#' @return Invisible copy of x
#' @rawNamespace S3method(base::print, parameter_audit)
print.parameter_audit <- function(x, ...) {
  cli::cli_h1("Parameter Info Audit")

  titles <- c(
    theta = "Theta Sources",
    omega = "Omega Sources",
    sigma = "Sigma Sources"
  )

  for (slot in names(titles)) {
    if (nrow(x[[slot]]) > 0) {
      print_data_table_console(x[[slot]], titles[[slot]])
    }
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

  titles <- c(
    theta = "Theta Sources",
    omega = "Omega Sources",
    sigma = "Sigma Sources"
  )

  for (slot in names(titles)) {
    if (nrow(x[[slot]]) > 0) {
      output <- c(output, print_data_table_knit(x[[slot]], titles[[slot]]))
    }
  }

  knitr::asis_output(paste(output, collapse = "\n"))
}
