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

  parts <- extract_raw_omega_parts("eta1 CL ; exp")
  expect_equal(parts$name, "eta1")
  expect_equal(parts$parameterization, "exp")
  expect_equal(parts$associated_theta, "CL")
})

test_that("parse omega comments works", {
	om_comment <- parse_raw_omega_comment("OMEGA(2,1)",	NULL, "OM2,1 CL-VC")
	expect_equal(om_comment@nonmem_name, "OMEGA(2,1)")
	expect_equal(om_comment@name, "OM2,1")
	expect_equal(om_comment@parameterization, NULL)
	expect_equal(om_comment@associated_theta, c("CL", "VC"))

	om_comment <- parse_raw_omega_comment("OMEGA(2,1)",	NULL, "OM2,1 CL-VC ;log")
	expect_equal(om_comment@nonmem_name, "OMEGA(2,1)")
	expect_equal(om_comment@name, "OM2,1")
	expect_equal(om_comment@parameterization, "LogNormal")
	expect_equal(om_comment@associated_theta, c("CL", "VC"))
})

