test_that("omega associated_theta matches theta names case-insensitively", {
  theta1 <- ThetaComment(
    nonmem_name = "THETA1",
    name = "CL/F"
  )
  omega11 <- OmegaComment(
    nonmem_name = "OMEGA(1,1)",
    name = "IIV",
    associated_theta = "cl/f"
  )

  info <- ModelComments(
    theta = list(THETA1 = theta1),
    omega = list(`OMEGA(1,1)` = omega11),
    sigma = list()
  )
  expect_equal(info@omega$`OMEGA(1,1)`@associated_theta, "CL/F")

  theta1 <- ThetaComment(
    nonmem_name = "THETA1",
    name = "cl/f"
  )
  omega11 <- OmegaComment(
    nonmem_name = "OMEGA(1,1)",
    name = "IIV",
    associated_theta = "CL/F"
  )

  info <- ModelComments(
    theta = list(THETA1 = theta1),
    omega = list(`OMEGA(1,1)` = omega11),
    sigma = list()
  )
  expect_equal(info@omega$`OMEGA(1,1)`@associated_theta, "cl/f")
})
