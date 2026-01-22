test_that("summary table gt snapshot from vignettes data", {
  testthat::skip_if_not_installed("gt")

  model_dir <- system.file("extdata", "models", "onecmt", package = "hyperion")
  testthat::skip_if_not(nzchar(model_dir), "Test data directory not found")

  tree <- get_model_lineage(model_dir)
  spec <- SummarySpec()

  table <- tree |>
    apply_summary_spec(spec) |>
    make_summary_table()

  snapshot_gt(table, "summary-table-base")
})

test_that("tag_filter selects tagged models", {
  model_dir <- system.file("extdata", "models", "onecmt", package = "hyperion")
  testthat::skip_if_not(nzchar(model_dir), "Test data directory not found")

  tree <- get_model_lineage(model_dir)
  for (name in names(tree$nodes)) {
    tree$nodes[[name]]$tags <- "draft"
  }
  tree$nodes[["run001.mod"]]$tags <- "final"

  spec <- SummarySpec(tag_filter = "final", columns = "ofv")

  data <- apply_summary_spec(tree, spec)

  expect_true(nrow(data) > 0)
  expect_true(all(data$model == "run001"))
})

test_that("summary table snapshot filtered to selected models", {
  testthat::skip_if_not_installed("gt")
  model_dir <- system.file("extdata", "models", "onecmt", package = "hyperion")
  testthat::skip_if_not(nzchar(model_dir), "Test data directory not found")

  tree <- get_model_lineage(model_dir)

  spec <- SummarySpec(
    models_to_include = c("run001", "run002", "run003")
  )

  table <- tree |>
    apply_summary_spec(spec) |>
    make_summary_table()

  snapshot_gt(table, "summary-table-filtered")
})

test_that("summary_filter applies to summary columns", {
  model_dir <- system.file("extdata", "models", "onecmt", package = "hyperion")
  testthat::skip_if_not(nzchar(model_dir), "Test data directory not found")

  tree <- get_model_lineage(model_dir)

  spec <- SummarySpec(
    columns = "ofv",
    summary_filter = summary_filter_rules(ofv == min(ofv, na.rm = TRUE))
  )

  data <- apply_summary_spec(tree, spec)

  expect_true(nrow(data) > 0)
  expect_true(all(data$ofv == min(data$ofv, na.rm = TRUE)))
})

test_that("models_to_include matches stems and extensions", {
  model_dir <- system.file("extdata", "models", "onecmt", package = "hyperion")
  testthat::skip_if_not(nzchar(model_dir), "Test data directory not found")

  tree <- get_model_lineage(model_dir)

  spec <- SummarySpec(
    columns = "ofv",
    models_to_include = c("run001", "run002.mod")
  )

  data <- apply_summary_spec(tree, spec)

  expect_true(nrow(data) > 0)
  expect_true(all(sort(unique(data$model)) == c("run001", "run002")))
})

test_that("remove_unrun_models snapshot", {
  testthat::skip_if_not_installed("gt")
  model_dir <- system.file("extdata", "models", "onecmt", package = "hyperion")
  testthat::skip_if_not(nzchar(model_dir), "Test data directory not found")

  tree <- get_model_lineage(model_dir)

  spec <- SummarySpec(remove_unrun_models = FALSE)

  table <- tree |>
    apply_summary_spec(spec) |>
    make_summary_table()

  snapshot_gt(table, "summary-table-include-unrun")
})

test_that("drop_columns removes description (snapshot)", {
  testthat::skip_if_not_installed("gt")
  model_dir <- system.file("extdata", "models", "onecmt", package = "hyperion")
  testthat::skip_if_not(nzchar(model_dir), "Test data directory not found")

  tree <- get_model_lineage(model_dir)

  spec <- SummarySpec(
    columns = c("description", "ofv"),
    drop_columns = "description"
  )

  table <- tree |>
    apply_summary_spec(spec) |>
    make_summary_table()

  snapshot_gt(table, "summary-table-drop-description")
})

test_that("time_format auto uses seconds label", {
  model_dir <- system.file("extdata", "models", "onecmt", package = "hyperion")
  testthat::skip_if_not(nzchar(model_dir), "Test data directory not found")

  tree <- get_model_lineage(model_dir)

  spec <- SummarySpec(
    add_columns = c("estimation_time", "covariance_time"),
    time_format = "auto"
  )

  data <- apply_summary_spec(tree, spec)

  expect_equal(attr(data, "summary_time_unit"), "s")

  table <- data |> make_summary_table()
  snapshot_gt(table, "summary-table-seconds-label")
})

test_that("time_format auto uses minutes label", {
  model_dir <- system.file("extdata", "models", "onecmt", package = "hyperion")
  testthat::skip_if_not(nzchar(model_dir), "Test data directory not found")

  tree <- get_model_lineage(model_dir)

  spec <- SummarySpec(
    columns = c("estimation_time", "covariance_time"),
    time_format = "auto"
  )

  data <- apply_summary_spec(tree, spec)

  data$estimation_time <- rep(120, nrow(data))
  data$covariance_time <- rep(90, nrow(data))
  data <- format_time_columns(data, spec)

  expect_equal(attr(data, "summary_time_unit"), "min")
  expect_true(all(data$estimation_time == 2))
})

test_that("time_format auto uses hours label", {
  model_dir <- system.file("extdata", "models", "onecmt", package = "hyperion")
  testthat::skip_if_not(nzchar(model_dir), "Test data directory not found")

  tree <- get_model_lineage(model_dir)

  spec <- SummarySpec(
    columns = c("estimation_time", "covariance_time"),
    time_format = "auto"
  )

  data <- apply_summary_spec(tree, spec)

  data$estimation_time <- rep(7200, nrow(data))
  data$covariance_time <- rep(5400, nrow(data))
  data <- format_time_columns(data, spec)

  expect_equal(attr(data, "summary_time_unit"), "h")
  expect_true(all(data$estimation_time == 2))
})

test_that("footnote_order NULL disables footnotes", {
  testthat::skip_if_not_installed("gt")
  model_dir <- system.file("extdata", "models", "onecmt", package = "hyperion")
  testthat::skip_if_not(nzchar(model_dir), "Test data directory not found")

  tree <- get_model_lineage(model_dir)

  spec <- SummarySpec(footnote_order = NULL)

  table <- tree |>
    apply_summary_spec(spec) |>
    make_summary_table()

  snapshot_gt(table, "summary-table-no-footnotes")
})

test_that("pvalue_threshold formats small p-values", {
  testthat::skip_if_not_installed("gt")
  model_dir <- system.file("extdata", "models", "onecmt", package = "hyperion")
  testthat::skip_if_not(nzchar(model_dir), "Test data directory not found")

  tree <- get_model_lineage(model_dir)

  spec <- SummarySpec(pvalue_threshold = 0.05)

  table <- tree |>
    apply_summary_spec(spec) |>
    make_summary_table()

  snapshot_gt(table, "summary-table-pval-thresh")
})
