
normalize_snapshot_lines <- function(lines) {
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

snapshot_print_output <- function(x, name) {
  expect_snapshot(
    cat(capture.output(print(x))),
    variant = name,
    transform = normalize_snapshot_lines
  )
}

snapshot_knit_html <- function(x, name) {
  out <- knitr::knit_print(x)
  html <- as.character(out)
  path <- file.path(tempdir(), paste0(name, ".html"))
  writeLines(html, path)
  expect_snapshot_file(path)
}

test_that("print and knit_print snapshots cover core classes", {
  model_dir <- system.file("extdata",
    "models",
    "onecmt",
    "run001",
    package = "hyperion"
  )
  mod_path <- system.file("extdata",
    "mod",
    "example1.mod",
    package = "hyperion"
  )

  mod <- read_model(mod_path)
  snapshot_print_output(mod, "model-print")
  snapshot_knit_html(mod, "model-knit")

  mod_sum <- get_model_summary(model_dir)
  snapshot_print_output(mod_sum, "summary-print")
  snapshot_knit_html(mod_sum, "summary-knit")

  info <- get_model_parameter_info(model_dir)
  snapshot_print_output(info, "comments-print")
  snapshot_knit_html(info, "comments-knit")

  audit <- audit_parameter_info(info)
  snapshot_print_output(audit, "audit-print")
  snapshot_knit_html(audit, "audit-knit")

  tree <- structure(
    list(
      nodes = list(
        "base.mod" = list(
          based_on = list(),
          description = "Base population PK model"
        ),
        "run001.mod" = list(
          based_on = list("base.mod"),
          description = "Run 1"
        ),
        "run002.mod" = list(
          based_on = list("run001.mod"),
          description = "Run 2 with covariate effects"
        )
      )
    ),
    class = "hyperion_nonmem_tree"
  )
  snapshot_print_output(tree, "tree-print")
  snapshot_knit_html(tree, "tree-knit")
})
