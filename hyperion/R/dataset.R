#' Print method for hyperion_nonmem_dataset objects
#'
#' @param x A hyperion_nonmem_dataset object
#' @param ... Additional arguments (ignored)
#' @return Invisible copy of x
#' @rawNamespace S3method(base::print, hyperion_nonmem_dataset)
print.hyperion_nonmem_dataset <- function(x, ...) {
  rel_path <- to_config_relative(x$canonical_path)

  cli::cli_h1("Dataset Check")
  cli::cli_text("{.strong Path:} {rel_path}")
  cli::cli_text("{.strong Hash:} {x$blake3_hash}")

  invisible(x)
}

#' Knit print method for hyperion_nonmem_dataset objects
#'
#' @param x A hyperion_nonmem_dataset object
#' @param ... Additional arguments (ignored)
#' @return HTML output for rendered documents
#' @exportS3Method knitr::knit_print
knit_print.hyperion_nonmem_dataset <- function(x, ...) {
  rel_path <- to_config_relative(x$canonical_path)

  df <- data.frame(
    Field = c("Path", "Hash"),
    Value = c(rel_path, x$blake3_hash),
    stringsAsFactors = FALSE
  )

  tbl <- knitr::kable(df, format = "html", col.names = NULL, escape = FALSE)

  output <- c(
    "<strong>Dataset Check</strong>",
    "",
    tbl
  )

  knitr::asis_output(paste(output, collapse = "\n"))
}
