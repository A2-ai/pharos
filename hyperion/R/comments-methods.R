#' Convert comment list to data frame with values
#' @param comments Named list of comment objects
#' @param fields Character vector of field names to extract
#' @return Data frame with parameter column and value columns
#' @noRd
comment_list_to_df <- function(comments, fields) {
  if (length(comments) == 0) {
    df <- data.frame(parameter = character(), stringsAsFactors = FALSE)
    for (f in fields) df[[f]] <- character()
    return(df)
  }

  rows <- lapply(names(comments), function(nm) {
    cmt <- comments[[nm]]
    row <- data.frame(parameter = nm, stringsAsFactors = FALSE)
    for (f in fields) {
      val <- S7::prop(cmt, f)
      if (is.null(val)) {
        row[[f]] <- NA_character_
      } else if (length(val) > 1) {
        row[[f]] <- paste(val, collapse = ", ")
      } else {
        row[[f]] <- val
      }
    }
    row
  })
  do.call(rbind, rows)
}

#' @noRd
S7::method(print, ModelComments) <- function(x, ...) {
  cli::cli_h1("Model Parameter Info")

  theta_df <- comment_list_to_df(
    x@theta,
    c("name", "display", "description", "unit", "parameterization")
  )
  omega_df <- comment_list_to_df(
    x@omega,
    c("name", "display", "description", "parameterization", "associated_theta")
  )
  sigma_df <- comment_list_to_df(
    x@sigma,
    c("name", "display", "description", "parameterization")
  )

  if (nrow(theta_df) > 0) {
    print_data_table_console(theta_df, "Theta Parameters")
  }
  if (nrow(omega_df) > 0) {
    print_data_table_console(omega_df, "Omega Parameters")
  }
  if (nrow(sigma_df) > 0) {
    print_data_table_console(sigma_df, "Sigma Parameters")
  }
  invisible(x)
}

#' @importFrom knitr knit_print
#' @noRd
S7::method(knit_print, ModelComments) <- function(x, ...) {
  output <- character()
  output <- c(output, "# Model Parameter Info", "")

  theta_df <- comment_list_to_df(
    x@theta,
    c("name", "display", "description", "unit", "parameterization")
  )
  omega_df <- comment_list_to_df(
    x@omega,
    c("name", "display", "description", "parameterization", "associated_theta")
  )
  sigma_df <- comment_list_to_df(
    x@sigma,
    c("name", "display", "description", "parameterization")
  )

  if (nrow(theta_df) > 0) {
    output <- c(output, print_data_table_knit(theta_df, "Theta Parameters"))
  }
  if (nrow(omega_df) > 0) {
    output <- c(output, print_data_table_knit(omega_df, "Omega Parameters"))
  }
  if (nrow(sigma_df) > 0) {
    output <- c(output, print_data_table_knit(sigma_df, "Sigma Parameters"))
  }

  knitr::asis_output(paste(output, collapse = "\n"))
}
