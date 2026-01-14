snapshot_gt_png <- function(table, name) {
  testthat::skip_if_not_installed("gt")
  testthat::skip_if_not_installed("webshot2")

  path <- file.path(tempdir(), paste0(name, ".png"))
  gt::gtsave(table, filename = path)

  testthat::expect_snapshot_file(path)
}
