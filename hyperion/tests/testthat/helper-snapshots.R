snapshot_knit_html <- function(x, name) {
  out <- knitr::knit_print(x)
  html <- as.character(out)
  html <- gsub("\\\\", "/", html)
  html <- gsub("[^[:space:]]*/extdata/", "extdata/", html)
  path <- file.path(tempdir(), paste0(name, ".html"))
  writeLines(html, path)
  expect_snapshot_file(path)
}

scrub_inst_path <- function(x) {
  gsub("[^[:space:]]*/extdata/", "extdata/", x)
}

scrub_audit_paths <- function(audit) {
  for (slot in c("theta", "omega", "sigma")) {
    if (!is.null(audit[[slot]]) && nrow(audit[[slot]]) > 0) {
      audit[[slot]][] <- lapply(audit[[slot]], function(col) {
        if (is.character(col)) {
          gsub("[^[:space:]]*/extdata/", "extdata/", col)
        } else {
          col
        }
      })
    }
  }
  audit
}
