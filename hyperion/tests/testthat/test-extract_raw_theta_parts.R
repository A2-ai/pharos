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
})

test_that("extract_raw_omega_parts parses comments correctly", {
  parts <- extract_raw_omega_parts("OMEGA1: CL :EXP")
  expect_equal(parts$name, "CL")
  expect_equal(parts$parameterization, "EXP")

  parts <- extract_raw_omega_parts("1: CL :EXP")
  expect_equal(parts$name, "CL")
  expect_equal(parts$parameterization, "EXP")

  parts <- extract_raw_omega_parts("1: OM2,1 CL-VC ; normal")
  expect_equal(parts$name, "OM2,1")
  expect_equal(parts$parameterization, "normal")
  expect_equal(parts$associated_theta, c("CL", "VC"))

  parts <- extract_raw_omega_parts("1: OM2,1 CL/VC ; normal")
  expect_equal(parts$name, "OM2,1")
  expect_equal(parts$parameterization, "normal")
  expect_equal(parts$associated_theta, c("CL", "VC"))
  
	parts <- extract_raw_omega_parts("OM2,1 CL,VC ; normal")
  expect_equal(parts$name, "OM2,1")
  expect_equal(parts$parameterization, "normal")
  expect_equal(parts$associated_theta, c("CL", "VC"))
	
	parts <- extract_raw_omega_parts("OMEGA1: CL ; exp")
  expect_equal(parts$name, "CL")
  expect_equal(parts$parameterization, "exp")
})
