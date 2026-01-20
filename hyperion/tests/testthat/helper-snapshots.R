normalize_snapshot_lines <- function(lines) {
  # Normalize path separators for cross-platform snapshots.
  lines <- gsub("\\\\", "/", lines)
  lines <- sub("[ \t]+$", "", lines)
  while (length(lines) > 0 && lines[1] == "") {
    lines <- lines[-1]
  }
  while (length(lines) > 0 && utils::tail(lines, 1) == "") {
    lines <- head(lines, -1)
  }
  if (length(lines) > 0) {
    out <- character()
    empty_run <- FALSE
    for (line in lines) {
      if (line == "") {
        if (!empty_run) {
          out <- c(out, line)
          empty_run <- TRUE
        }
      } else {
        out <- c(out, line)
        empty_run <- FALSE
      }
    }
    lines <- out
  }
  lines
}

snapshot_knit_html <- function(x, name) {
  out <- knitr::knit_print(x)
  html <- as.character(out)
  html <- gsub("\\\\", "/", html)
  path <- file.path(tempdir(), paste0(name, ".html"))
  writeLines(html, path)
  expect_snapshot_file(path)
}
