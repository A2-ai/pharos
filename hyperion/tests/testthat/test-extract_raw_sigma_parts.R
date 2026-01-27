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

test_that("extract_raw_sigma_parts captures units in parentheses or brackets", {
  parts <- extract_raw_sigma_parts("22 AddErr (CONC)")
  expect_equal(parts$name, "AddErr")
  expect_equal(parts$unit, "CONC")
  expect_equal(parts$parameterization, NULL)

  parts <- extract_raw_sigma_parts("AddErr [ng/mL] :ADD")
  expect_equal(parts$name, "AddErr")
  expect_equal(parts$unit, "ng/mL")
  expect_equal(parts$parameterization, "ADD")

  parts <- extract_raw_sigma_parts("AddErr ;AddErr (ng/mL)")
  expect_equal(parts$name, "AddErr")
  expect_equal(parts$unit, "ng/mL")
  expect_equal(parts$parameterization, "AddErr")
})
