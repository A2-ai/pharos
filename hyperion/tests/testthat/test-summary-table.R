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

test_that("tag_filter selects tagged models", {
  model_dir <- system.file(
    "extdata",
    "test_data",
    "models",
    "onecmt",
    package = "hyperion"
  )
  testthat::skip_if_not(nzchar(model_dir), "Test data directory not found")

  tree <- get_model_lineage(model_dir)
  for (name in names(tree$nodes)) {
    tree$nodes[[name]]$tags <- "draft"
  }
  tree$nodes[["run001.mod"]]$tags <- "final"

  spec <- SummarySpec(tag_filter = "final", fields = "ofv")

  data <- apply_summary_spec(tree, spec)

  expect_true(nrow(data) > 0)
  expect_true(all(data$model == "run001"))
})

test_that("summary table snapshot filtered to selected models", {
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

  spec <- SummarySpec(
    models_to_include = c("run001", "run002", "run003")
  )

  table <- tree |>
    apply_summary_spec(spec) |>
    make_summary_table()

  snapshot_gt_png(table, "summary-table-filtered")
})

test_that("summary_filter applies to summary columns", {
  model_dir <- system.file(
    "extdata",
    "test_data",
    "models",
    "onecmt",
    package = "hyperion"
  )
  testthat::skip_if_not(nzchar(model_dir), "Test data directory not found")

  tree <- get_model_lineage(model_dir)

  spec <- SummarySpec(
    fields = "ofv",
    summary_filter = summary_filter_rules(ofv == min(ofv, na.rm = TRUE))
  )

  data <- apply_summary_spec(tree, spec)

  expect_true(nrow(data) > 0)
  expect_true(all(data$ofv == min(data$ofv, na.rm = TRUE)))
})

test_that("models_to_include matches stems and extensions", {
  model_dir <- system.file(
    "extdata",
    "test_data",
    "models",
    "onecmt",
    package = "hyperion"
  )
  testthat::skip_if_not(nzchar(model_dir), "Test data directory not found")

  tree <- get_model_lineage(model_dir)

  spec <- SummarySpec(
    fields = "ofv",
    models_to_include = c("run001", "run002.mod")
  )

  data <- apply_summary_spec(tree, spec)

  expect_true(nrow(data) > 0)
  expect_true(all(sort(unique(data$model)) == c("run001", "run002")))
})

test_that("remove_unrun_models snapshot", {
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

  spec <- SummarySpec(remove_unrun_models = FALSE)

  table <- tree |>
    apply_summary_spec(spec) |>
    make_summary_table()

  snapshot_gt_png(table, "summary-table-include-unrun")
})

test_that("drop_columns removes description (snapshot)", {
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

  spec <- SummarySpec(
    fields = c("description", "ofv"),
    drop_columns = "description"
  )

  table <- tree |>
    apply_summary_spec(spec) |>
    make_summary_table()

  snapshot_gt_png(table, "summary-table-drop-description")
})
