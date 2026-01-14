test_that("extract_raw_sigma_parts skips numbered descriptions with colon", {
  parts <- extract_raw_sigma_parts("1: Proportional error")
  expect_equal(parts$name, NULL)
  expect_equal(parts$parameterization, NULL)
})
