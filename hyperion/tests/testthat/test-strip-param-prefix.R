test_that("lowercase parameter prefixes are stripped", {
  theta_parts <- extract_raw_theta_parts("theta1: CL (L/h)")
  expect_equal(theta_parts$name, "CL")
  expect_equal(theta_parts$unit, "L/h")

  omega_parts <- extract_raw_omega_parts("omega1: IIV CL ; exp")
  expect_equal(omega_parts$name, "IIV")
  expect_equal(omega_parts$associated_theta, "CL")
})
