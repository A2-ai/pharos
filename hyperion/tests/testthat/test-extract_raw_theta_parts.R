test_that("extract_raw_theta_parts parses comments correctly", {
  parts <- extract_raw_theta_parts("THETA1: CL (L/day) ; exp")
  expect_equal(parts$name, "CL")
  expect_equal(parts$unit, "L/day")
  expect_equal(parts$parameterization, "exp")

  parts <- extract_raw_theta_parts("1: CL (L/day) ; exp")
  expect_equal(parts$name, "CL")
  expect_equal(parts$unit, "L/day")
  expect_equal(parts$parameterization, "exp")

  parts <- extract_raw_theta_parts("1 CL (L/day) ; exp")
  expect_equal(parts$name, "CL")
  expect_equal(parts$unit, "L/day")
  expect_equal(parts$parameterization, "exp")

  parts <- extract_raw_theta_parts("CL (L/day) ; exp")
  expect_equal(parts$name, "CL")
  expect_equal(parts$unit, "L/day")
  expect_equal(parts$parameterization, "exp")

  parts <- extract_raw_theta_parts("CL ;exp")
  expect_equal(parts$name, "CL")
  expect_equal(parts$unit, NULL)
  expect_equal(parts$parameterization, "exp")

  parts <- extract_raw_theta_parts("THETA1: CL (L/day) :EXP")
  expect_equal(parts$name, "CL")
  expect_equal(parts$unit, "L/day")
  expect_equal(parts$parameterization, "EXP")

  parts <- extract_raw_theta_parts("THETA1 CL (L/day) :LOG")
  expect_equal(parts$name, "CL")
  expect_equal(parts$unit, "L/day")
  expect_equal(parts$parameterization, "LOG")

  parts <- extract_raw_theta_parts("THETA6 RUV :ADD")
  expect_equal(parts$name, "RUV")
  expect_equal(parts$unit, NULL)
  expect_equal(parts$parameterization, "ADD")
})
