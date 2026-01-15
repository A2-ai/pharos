test_that("SummarySpec validates fields", {
  expect_error(
    SummarySpec(fields = c("invalid_field")),
    "@fields must be in"
  )

  expect_error(
    SummarySpec(time_format = "invalid"),
    "@time_format must be"
  )

  expect_error(
    SummarySpec(n_sigfig = 0),
    "@n_sigfig must be a positive"
  )

  # Valid spec should not error
  spec <- SummarySpec(
    fields = c("number_obs", "ofv"),
    time_format = "minutes",
    n_sigfig = 4
  )
  expect_true(S7::S7_inherits(spec, SummarySpec))
})

test_that("summary_filter_rules creates quosures", {
  rules <- summary_filter_rules(
    "final" %in% tags,
    !is.null(description)
  )
  expect_true(all(vapply(rules, rlang::is_quosure, logical(1))))
})

test_that("apply_summary_spec requires hyperion_nonmem_tree", {
  expect_error(
    apply_summary_spec(data.frame()),
    "tree must be a hyperion_nonmem_tree"
  )

  expect_error(
    apply_summary_spec(list(nodes = list()), SummarySpec()),
    "tree must be a hyperion_nonmem_tree"
  )
})

test_that("apply_summary_spec works with lineage tree", {
  model_dir <- testthat::test_path("testdata", "models", "onecmt")

  skip_if_not(dir.exists(model_dir), "Test data directory not found")

  tree <- get_model_lineage(model_dir)

  expect_true(inherits(tree, "hyperion_nonmem_tree"))
  expect_true(!is.null(tree$source_dir) && tree$source_dir != "")

  # Apply spec with default options
  spec <- SummarySpec()
  result <- apply_summary_spec(tree, spec)

  expect_true(is.data.frame(result))
  expect_true("model" %in% names(result))
  expect_true(!is.null(attr(result, "summary_spec")))
})

test_that("apply_summary_spec filters models correctly", {
  model_dir <- testthat::test_path("testdata", "models", "onecmt")

  skip_if_not(dir.exists(model_dir), "Test data directory not found")

  tree <- get_model_lineage(model_dir)

  # Filter to specific models
  spec <- SummarySpec(
    model_filter = summary_filter_rules(
      name %in% c("run001.mod", "run002.mod")
    )
  )

  result <- apply_summary_spec(tree, spec)

  expect_true(nrow(result) <= 2)
  if (nrow(result) > 0) {
    expect_true(all(grepl("run001|run002", result$model)))
  }
})

test_that("apply_summary_spec respects field selection", {
  model_dir <- testthat::test_path("testdata", "models", "onecmt")

  skip_if_not(dir.exists(model_dir), "Test data directory not found")

  tree <- get_model_lineage(model_dir)

  spec <- SummarySpec(
    fields = c("number_obs", "ofv")
  )

  result <- apply_summary_spec(tree, spec)

  expect_true("model" %in% names(result))
  expect_true("number_obs" %in% names(result))
  expect_true("ofv" %in% names(result))
  expect_false("estimation_method" %in% names(result))
})

test_that("make_summary_table creates gt table", {
  model_dir <- testthat::test_path("testdata", "models", "onecmt")

  skip_if_not(dir.exists(model_dir), "Test data directory not found")
  skip_if_not_installed("gt")

  tree <- get_model_lineage(model_dir)

  spec <- SummarySpec(
    model_filter = summary_filter_rules(
      name %in% c("run001.mod", "run002.mod", "run003.mod")
    ),
    fields = c("number_obs", "estimation_method", "ofv"),
    title = "Test Summary Table"
  )

  result <- tree |>
    apply_summary_spec(spec) |>
    make_summary_table()

  expect_true(inherits(result, "gt_tbl"))
})

test_that("get_summary_spec retrieves attached spec", {
  model_dir <- testthat::test_path("testdata", "models", "onecmt")

  skip_if_not(dir.exists(model_dir), "Test data directory not found")

  tree <- get_model_lineage(model_dir)

  spec <- SummarySpec(title = "Custom Title")
  result <- apply_summary_spec(tree, spec)

  retrieved_spec <- get_summary_spec(result)

  expect_true(S7::S7_inherits(retrieved_spec, SummarySpec))
  expect_equal(retrieved_spec@title, "Custom Title")
})
