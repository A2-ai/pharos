test_that("compute_cv recycles scalar param_type to estimate length", {
  estimate <- c(0.09, 0.16)
  res <- compute_cv(estimate, "Omega", "LogNormal")
  expect_length(res, length(estimate))
})

test_that("compute_rse recycles scalar param_type to estimate length", {
  estimate <- c(1.0, 2.0, 3.0)
  se <- c(0.1, 0.2, 0.3)
  res <- compute_rse(estimate, se, "Theta", "Identity")
  expect_length(res, length(estimate))
})
