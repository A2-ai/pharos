test_that("parse omega comments works", {
  om_comment <- parse_raw_omega_comment("OMEGA(2,1)", NULL, "OM2,1 CL-VC")
  expect_equal(om_comment@nonmem_name, "OMEGA(2,1)")
  expect_equal(om_comment@name, "OM2,1")
  expect_equal(om_comment@parameterization, NULL)
  expect_equal(om_comment@associated_theta, c("CL", "VC"))

  om_comment <- parse_raw_omega_comment("OMEGA(2,1)", NULL, "OM2,1 CL-VC ;log")
  expect_equal(om_comment@nonmem_name, "OMEGA(2,1)")
  expect_equal(om_comment@name, "OM2,1")
  expect_equal(om_comment@parameterization, "LogNormal")
  expect_equal(om_comment@associated_theta, c("CL", "VC"))
})
