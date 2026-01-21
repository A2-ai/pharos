snapshot_gt <- function(table, name) {
  testthat::skip_if_not_installed("gt")
  testthat::skip_if_not_installed("webshot2")

  path <- file.path(tempdir(), paste0(name, ".png"))
  gt::gtsave(table, filename = path, vwidth = 4000)

  testthat::expect_snapshot_file(path)
}
