test_that("extract_raw_sigma_parts accepts numbered prefixes", {
  parts <- extract_raw_sigma_parts("1: Proportional error")
  expect_equal(parts$name, "Proportional")
  expect_equal(parts$parameterization, NULL)
})

test_that("extract_raw_sigma_parts handles numbered name comments", {
  parts <- extract_raw_sigma_parts("11 PropErr ;Proportional")
  expect_equal(parts$name, "PropErr")
  expect_equal(parts$parameterization, "Proportional")
})

test_that("extract_raw_sigma_parts handles numbered name comments", {
  parts <- extract_raw_sigma_parts("11 PropErr :Proportional")
  expect_equal(parts$name, "PropErr")
  expect_equal(parts$parameterization, "Proportional")
})

test_that("extract_raw_sigma_parts handles numbered name comments", {
  parts <- extract_raw_sigma_parts("SIGMA2 AddErr :AddErr")
  expect_equal(parts$name, "AddErr")
  expect_equal(parts$parameterization, "AddErr")
})
