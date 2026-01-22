#' @noRd
S7::method(print, ModelComments) <- function(x, ...) {
  cli::cli_text("")
  cli::cli_h1("Model Parameter Info")

  tables <- build_comment_tables(
    list(theta = x@theta, omega = x@omega, sigma = x@sigma),
    list(
      theta = theta_fields(),
      omega = omega_fields(),
      sigma = sigma_fields()
    ),
    function(cmt, field) S7::prop(cmt, field)
  )

  titles <- c(
    theta = "Theta Parameters",
    omega = "Omega Parameters",
    sigma = "Sigma Parameters"
  )

  for (slot in names(titles)) {
    if (nrow(tables[[slot]]) > 0) {
      print_data_table_console(tables[[slot]], titles[[slot]])
    }
  }
  invisible(x)
}

#' @importFrom knitr knit_print
#' @noRd
S7::method(knit_print, ModelComments) <- function(x, ...) {
  output <- character()
  output <- c(
    output,
    "",
    '<strong>Model Parameter Info</strong>',
    ""
  )

  tables <- build_comment_tables(
    list(theta = x@theta, omega = x@omega, sigma = x@sigma),
    list(
      theta = theta_fields(),
      omega = omega_fields(),
      sigma = sigma_fields()
    ),
    function(cmt, field) S7::prop(cmt, field)
  )

  titles <- c(
    theta = "Theta Parameters",
    omega = "Omega Parameters",
    sigma = "Sigma Parameters"
  )

  for (slot in names(titles)) {
    if (nrow(tables[[slot]]) > 0) {
      output <- c(
        output,
        "",
        print_data_table_knit(tables[[slot]], titles[[slot]])
      )
    }
  }

  knitr::asis_output(paste(output, collapse = "\n"))
}
