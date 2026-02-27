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

test_that("process_heuristics_data handles NA values with 3-way messages", {
  run_heuristics <- data.frame(
    heuristic_name = c(
      "minimization_terminated",
      "covariance_step_aborted",
      "eigenvalue_issues",
      "parameter_near_boundary",
      "hessian_reset"
    ),
    value = c(FALSE, NA, NA, TRUE, NA),
    stringsAsFactors = FALSE
  )

  result <- process_heuristics_data(run_heuristics)

  expect_equal(nrow(result), 5)
  expect_equal(
    result$message,
    c(
      "Minimization Successful",
      "Covariance Step Not Run",
      "Eigenvalue Check Not Available",
      "Parameters Near Boundary",
      "Hessian Reset Check Not Available"
    )
  )
  expect_equal(
    result$has_issue,
    c(FALSE, NA, NA, TRUE, NA)
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
