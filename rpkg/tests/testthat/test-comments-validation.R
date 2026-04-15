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

test_that("omega associated_theta matches theta names by stripping suffix", {
  theta1 <- ThetaComment(
    nonmem_name = "THETA1",
    name = "CL/F"
  )
  theta2 <- ThetaComment(
    nonmem_name = "THETA2",
    name = "VC/F"
  )
  omega11 <- OmegaComment(
    nonmem_name = "OMEGA(1,1)",
    name = "IIV-CL",
    associated_theta = "CL"
  )
  omega22 <- OmegaComment(
    nonmem_name = "OMEGA(2,2)",
    name = "IIV-VC",
    associated_theta = "VC"
  )

  info <- ModelComments(
    theta = list(THETA1 = theta1, THETA2 = theta2),
    omega = list(`OMEGA(1,1)` = omega11, `OMEGA(2,2)` = omega22),
    sigma = list()
  )

  expect_equal(info@omega$`OMEGA(1,1)`@associated_theta, "CL/F")
  expect_equal(info@omega$`OMEGA(2,2)`@associated_theta, "VC/F")
})
