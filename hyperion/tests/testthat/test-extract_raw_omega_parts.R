test_that("extract_raw_omega_parts parses comments correctly", {
  # Single word after prefix stripping - no theta ref, just name
  parts <- extract_raw_omega_parts("OMEGA1: CL :EXP")
  expect_equal(parts$name, "CL")
  expect_equal(parts$parameterization, "EXP")

  parts <- extract_raw_omega_parts("1: CL :EXP")
  expect_equal(parts$name, "CL")
  expect_equal(parts$parameterization, "EXP")

  # Name is prefix only, associated_theta stored separately
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

  # Single word - no theta ref
  parts <- extract_raw_omega_parts("OMEGA1: CL ; exp")
  expect_equal(parts$name, "CL")
  expect_equal(parts$parameterization, "exp")

  # prefix + theta ref = prefix in name, theta in associated_theta
  parts <- extract_raw_omega_parts("eta1 CL ; exp")
  expect_equal(parts$name, "eta1")
  expect_equal(parts$parameterization, "exp")
  expect_equal(parts$associated_theta, "CL")
})
