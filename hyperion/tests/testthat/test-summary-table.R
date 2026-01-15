test_that("summary table gt snapshot from vignettes data", {
  testthat::skip_if_not_installed("gt")

  model_dir <- system.file(
    "extdata",
    "test_data",
    "models",
    "onecmt",
    package = "hyperion"
  )
  testthat::skip_if_not(nzchar(model_dir), "Test data directory not found")

  tree <- get_model_lineage(model_dir)
  spec <- SummarySpec()

  table <- tree |>
    apply_summary_spec(spec) |>
    make_summary_table()

  snapshot_gt_png(table, "summary-table-base")
})
