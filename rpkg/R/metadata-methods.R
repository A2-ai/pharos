#' Print method for hyperion_model_metadata objects
#'
#' @param x A hyperion_model_metadata object
#' @param ... Additional arguments (ignored)
#' @return Invisible copy of x
#' @rawNamespace S3method(base::print, hyperion_model_metadata)
print.hyperion_model_metadata <- function(x, ...) {
  description <- x$description %||% ""
  tags <- x$tags %||% character(0)
  based_on <- x$based_on %||% character(0)

  tags_text <- if (length(tags) == 0) "(none)" else paste(tags, collapse = ", ")
  based_on_text <- if (length(based_on) == 0) {
    "(none)"
  } else {
    paste(based_on, collapse = ", ")
  }
  description_text <- if (nzchar(trimws(description))) description else "(none)"

  cli::cli_h1("Model Metadata")
  cli::cli_text("{.strong Description:} {description_text}")
  cli::cli_text("{.strong Tags:} {tags_text}")
  cli::cli_text("{.strong Based On:} {based_on_text}")

  invisible(x)
}

#' Knit print method for hyperion_model_metadata objects
#'
#' @param x A hyperion_model_metadata object
#' @param ... Additional arguments (ignored)
#' @return HTML output for rendered documents
#' @exportS3Method knitr::knit_print
knit_print.hyperion_model_metadata <- function(x, ...) {
  description <- x$description %||% ""
  tags <- x$tags %||% character(0)
  based_on <- x$based_on %||% character(0)

  tags_text <- if (length(tags) == 0) "(none)" else paste(tags, collapse = ", ")
  based_on_text <- if (length(based_on) == 0) {
    "(none)"
  } else {
    paste(based_on, collapse = ", ")
  }
  description_text <- if (nzchar(trimws(description))) description else "(none)"

  df <- data.frame(
    Field = c("Description", "Tags", "Based On"),
    Value = c(description_text, tags_text, based_on_text),
    stringsAsFactors = FALSE
  )

  tbl <- knitr::kable(df, format = "html", col.names = NULL, escape = FALSE)
  output <- c("<strong>Model Metadata</strong>", "", tbl)

  knitr::asis_output(paste(output, collapse = "\n"))
}
