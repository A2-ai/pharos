snapshot_knit_html <- function(x, name) {
  out <- knitr::knit_print(x)
  html <- as.character(out)
  html <- gsub("\\\\", "/", html)
  path <- file.path(tempdir(), paste0(name, ".html"))
  writeLines(html, path)
  expect_snapshot_file(path)
}
