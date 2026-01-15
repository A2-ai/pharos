summary_table_tree <- function() {
  model_dir <- testthat::test_path("testdata", "models", "onecmt")
  testthat::skip_if_not(dir.exists(model_dir), "Test data directory not found")
  get_model_lineage(model_dir)
}

test_that("summary table gt snapshot", {
  tree <- summary_table_tree()
  testthat::skip_if_not_installed("gt")

  spec <- SummarySpec(
    model_filter = summary_filter_rules(
      name %in% c("run002.mod", "run003.mod", "run003b1.mod")
    ),
    fields = c("number_obs", "estimation_method", "ofv"),
    title = "Test Summary Table"
  )

  table <- tree |>
    apply_summary_spec(spec) |>
    make_summary_table()

  expect_true(inherits(table, "gt_tbl"))
  snapshot_gt_png(table, "summary-table-base")
})
