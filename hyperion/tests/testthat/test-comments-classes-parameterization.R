test_that("parameterization values are validated and normalized", {
  expect_error(
    ThetaComment(nonmem_name = "THETA1", parameterization = "BAD"),
    "must be one of"
  )

  theta <- ThetaComment(
    nonmem_name = "THETA1",
    parameterization = "lognormal"
  )
  expect_equal(theta@parameterization, "LogNormal")
})
