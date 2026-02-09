test_that("load_summary_config_thresholds falls back on error", {
  testthat::local_mocked_bindings(
    get_pharos_config = function() rlang::abort("no config")
  )

  expect_warning(
    thresholds <- load_summary_config_thresholds(),
    "pharos.toml file could not be found"
  )

  expect_equal(thresholds$correlation_threshold, 0.95)
  expect_equal(thresholds$condition_threshold, 1000)
})

test_that("process_heuristics_data returns empty for no rows", {
  result <- process_heuristics_data(data.frame())
  expect_equal(nrow(result), 0)
})

test_that("filter_and_sort_correlations returns empty on NULL", {
  result <- filter_and_sort_correlations(NULL, 0.9)
  expect_equal(nrow(result$correlations), 0)
  expect_null(result$method)
})

test_that("filter_and_sort_correlations returns empty when below threshold", {
  corr <- data.frame(
    param1 = c("CL", "V"),
    param2 = c("V", "CL"),
    correlation = c(0.5, -0.6),
    method = c("FOCE", "FOCE"),
    stringsAsFactors = FALSE
  )

  result <- filter_and_sort_correlations(corr, 0.9)
  expect_equal(nrow(result$correlations), 0)
  expect_null(result$method)
})
