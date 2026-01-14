test_that("omega display name does not duplicate associated theta names", {
  theta1 <- ThetaComment(
    nonmem_name = "THETA1",
    name = "CL"
  )
  theta2 <- ThetaComment(
    nonmem_name = "THETA2",
    name = "V"
  )

  omega_partial <- OmegaComment(
    nonmem_name = "OMEGA(2,1)",
    name = "Corr-CL",
    associated_theta = c("CL", "V")
  )
  omega_full <- OmegaComment(
    nonmem_name = "OMEGA(2,2)",
    name = "Corr-CL-V",
    associated_theta = c("CL", "V")
  )

  info <- ModelComments(
    theta = list(THETA1 = theta1, THETA2 = theta2),
    omega = list(`OMEGA(2,1)` = omega_partial, `OMEGA(2,2)` = omega_full),
    sigma = list()
  )

  lookup <- hyperion:::build_name_lookup(info, "name")

  partial_display <- lookup$display[
    lookup$key == "OMEGA(2,1)" & lookup$kind == "OMEGA"
  ]
  full_display <- lookup$display[
    lookup$key == "OMEGA(2,2)" & lookup$kind == "OMEGA"
  ]

  expect_equal(partial_display, "Corr-CL-V")
  expect_equal(full_display, "Corr-CL-V")
})

test_that("nonmem_name source uses theta NONMEM names for omega associations", {
  theta1 <- ThetaComment(
    nonmem_name = "THETA1",
    name = "TVCL"
  )
  theta2 <- ThetaComment(
    nonmem_name = "THETA2",
    name = "TVV"
  )
  omega11 <- OmegaComment(
    nonmem_name = "OMEGA(1,1)",
    name = "IIV",
    associated_theta = "TVCL"
  )

  info <- ModelComments(
    theta = list(THETA1 = theta1, THETA2 = theta2),
    omega = list(`OMEGA(1,1)` = omega11),
    sigma = list()
  )

  lookup <- hyperion:::build_name_lookup(info, "nonmem_name")
  omega_display <- lookup$display[
    lookup$key == "OMEGA(1,1)" & lookup$kind == "OMEGA"
  ]

  expect_equal(omega_display, "OMEGA(1,1)-THETA1")
})
