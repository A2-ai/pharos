test_that("compute_cv enforces length checks with length-1 recycling", {
  estimate <- c(0.09, 0.16, 0.25)

  # transform length-1 should recycle
  res <- compute_cv(estimate, "Omega", "LogNormal")
  expect_length(res, length(estimate))

  # transform length mismatch should error (length != 1 and != estimate length)
  expect_error(
    compute_cv(estimate, "Omega", c("LogNormal", "Identity")),
    "transform length",
    fixed = FALSE
  )
})

test_that("compute_rse enforces length checks with length-1 recycling", {
  estimate <- c(1.0, 2.0, 3.0)
  se <- c(0.1, 0.2, 0.3)

  # transform length-1 should recycle
  res <- compute_rse(estimate, se, "Theta", "Identity")
  expect_length(res, length(estimate))

  # se length mismatch should error
  expect_error(
    compute_rse(estimate, c(0.1, 0.2), "Theta", "Identity"),
    "se length",
    fixed = FALSE
  )

  # transform length mismatch should error
  expect_error(
    compute_rse(estimate, se, "Theta", c("Identity", "LogNormal")),
    "transform length",
    fixed = FALSE
  )
})

test_that("compute_ci enforces length checks with length-1 recycling", {
  estimate <- c(1.5, 2.0, 2.5)
  se <- c(0.2, 0.3, 0.4)

  # transform length-1 should recycle
  res <- compute_ci(estimate, se, 0.95, "Identity")
  expect_true(is.list(res))
  expect_length(res$lower, length(estimate))
  expect_length(res$upper, length(estimate))

  # se length mismatch should error
  expect_error(
    compute_ci(estimate, c(0.2, 0.3), 0.95, "Identity"),
    "se length",
    fixed = FALSE
  )

  # transform length mismatch should error
  expect_error(
    compute_ci(estimate, se, 0.95, c("Identity", "LogNormal")),
    "transform length",
    fixed = FALSE
  )
})
