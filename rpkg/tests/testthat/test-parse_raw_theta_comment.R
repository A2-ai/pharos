test_that("parse theta comments works", {
  th <- parse_raw_theta_comment("THETA1", NULL, "THETA1 RUV :ADD")
  expect_equal(th@nonmem_name, "THETA1")
  expect_equal(th@name, "RUV")
  expect_equal(th@parameterization, "AddErr")

  th <- parse_raw_theta_comment("THETA2", NULL, "THETA2 CL :log")
  expect_equal(th@nonmem_name, "THETA2")
  expect_equal(th@name, "CL")
  expect_equal(th@parameterization, "LogNormal")
})
