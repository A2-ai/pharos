test_that("parameter prefixes with parentheses are stripped", {
  theta_parts <- extract_raw_theta_parts("THETA(1): CL (L/h)")
  expect_equal(theta_parts$name, "CL")
  expect_equal(theta_parts$unit, "L/h")

  sigma_parts <- extract_raw_sigma_parts("SIGMA(1): PropErr ; prop")
  expect_equal(sigma_parts$name, "PropErr")
  expect_equal(sigma_parts$parameterization, "prop")
})
