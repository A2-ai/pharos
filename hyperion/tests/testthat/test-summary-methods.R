test_that("process_heuristics_data orders and labels results", {
  run_heuristics <- data.frame(
    heuristic_name = c("covariance_step_aborted", "minimization_terminated"),
    value = c(TRUE, FALSE),
    stringsAsFactors = FALSE
  )

  result <- process_heuristics_data(run_heuristics)

  expect_equal(
    result$heuristic,
    c("minimization_terminated", "covariance_step_aborted")
  )
  expect_equal(
    result$message,
    c("Minimization Successful", "Covariance Step Aborted")
  )
})

test_that("filter_and_sort_correlations filters and sorts by abs value", {
  corr <- data.frame(
    param1 = c("CL", "V", "KA"),
    param2 = c("V", "KA", "CL"),
    correlation = c(0.8, -0.96, 0.91),
    method = c("FOCE", "FOCE", "FOCE"),
    stringsAsFactors = FALSE
  )

  result <- filter_and_sort_correlations(corr, 0.9)

  expect_equal(result$method, "FOCE")
  expect_equal(result$correlations$correlation[1], -0.96)
  expect_equal(nrow(result$correlations), 2)
})
