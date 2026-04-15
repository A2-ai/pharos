test_that("split_theta_reference trims whitespace around separators", {
  result <- split_theta_reference("CL / V")
  expect_equal(result, c("CL", "V"))

  result <- split_theta_reference("CL, V")
  expect_equal(result, c("CL", "V"))
})
