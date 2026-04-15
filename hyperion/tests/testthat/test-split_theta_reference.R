test_that("split_theta_reference respects known thetas", {
  # No known thetas - splits
  expect_equal(split_theta_reference("CL/F"), c("CL", "F"))
  expect_equal(split_theta_reference("CL-V"), c("CL", "V"))
  expect_equal(split_theta_reference("CL"), "CL")

  # Known theta - keeps as-is
  expect_equal(split_theta_reference("CL/F", c("CL/F", "V")), "CL/F")

  # Case-insensitive match
  expect_equal(split_theta_reference("cl/f", c("CL/F")), "cl/f")

  # No match in known - splits
  expect_equal(split_theta_reference("CL/V", c("CL/F", "KA")), c("CL", "V"))
})
