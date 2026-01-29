#' Make path relative to pharos.toml directory
#'
#' @param path Absolute path to make relative
#' @return Path relative to pharos.toml directory, or original path if not possible
#' @noRd
make_pharos_relative_path <- function(path) {
  tryCatch(
    {
      config_path <- find_pharos_config_file()
      if (grepl("No pharos.toml", config_path)) {
        return(path)
      }
      config_dir <- dirname(config_path)

      # Check if path starts with config_dir
      if (startsWith(path, config_dir)) {
        rel_path <- substring(path, nchar(config_dir) + 2) # +2 for trailing /
        return(rel_path)
      }
      path
    },
    error = function(e) path
  )
}

#' Print method for hyperion_nonmem_dataset objects
#'
#' @param x A hyperion_nonmem_dataset object
#' @param ... Additional arguments (ignored)
#' @return Invisible copy of x
#' @rawNamespace S3method(base::print, hyperion_nonmem_dataset)
print.hyperion_nonmem_dataset <- function(x, ...) {
  rel_path <- make_pharos_relative_path(x$canonical_path)

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
  rel_path <- make_pharos_relative_path(x$canonical_path)

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
