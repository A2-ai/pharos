test_that("get_parameter_names maps NONMEM names to name/display", {
  theta1 <- ThetaComment(
    nonmem_name = "THETA1",
    name = "CL",
    display = "CL (L/h)"
  )
  omega11 <- OmegaComment(
    nonmem_name = "OMEGA(1,1)",
    name = "IIV-CL"
  )
  sigma11 <- SigmaComment(
    nonmem_name = "SIGMA(1,1)",
    name = "PropErr"
  )

  info <- ModelComments(
    theta = list(THETA1 = theta1),
    omega = list(`OMEGA(1,1)` = omega11),
    sigma = list(`SIGMA(1,1)` = sigma11)
  )

  names_df <- get_parameter_names(info)

  expect_equal(rownames(names_df), c("THETA1", "OMEGA(1,1)", "SIGMA(1,1)"))
  expect_equal(names_df["THETA1", "name"], "CL")
  expect_equal(names_df["THETA1", "display"], "CL (L/h)")
  expect_true(is.na(names_df["OMEGA(1,1)", "display"]))
})

test_that("get_parameter_names uses associated_theta when name is NA", {
  theta1 <- ThetaComment(
    nonmem_name = "THETA1",
    name = "CL/F"
  )
  omega11 <- OmegaComment(
    nonmem_name = "OMEGA(1,1)",
    name = NA_character_,
    associated_theta = "CL/F"
  )

  info <- ModelComments(
    theta = list(THETA1 = theta1),
    omega = list(`OMEGA(1,1)` = omega11),
    sigma = list()
  )

  names_df <- get_parameter_names(info)

  expect_equal(names_df["OMEGA(1,1)", "name"], "CL/F")
})

test_that("get_parameter_unit/transform resolve names and NONMEM ids", {
  theta1 <- ThetaComment(
    nonmem_name = "THETA1",
    name = "CL",
    unit = "L/h",
    parameterization = "LogNormal"
  )
  sigma11 <- SigmaComment(
    nonmem_name = "SIGMA(1,1)",
    name = "AddErr",
    unit = "ng/mL"
  )
  omega11 <- OmegaComment(
    nonmem_name = "OMEGA(1,1)",
    name = "IIV-CL",
    parameterization = "LogNormal"
  )

  info <- ModelComments(
    theta = list(THETA1 = theta1),
    omega = list(`OMEGA(1,1)` = omega11),
    sigma = list(`SIGMA(1,1)` = sigma11)
  )

  transforms <- get_parameter_transform(
    info,
    c("THETA1", "IIV-CL", "OMEGA(1,1)", "UNKNOWN")
  )
  units <- get_parameter_unit(
    info,
    c("THETA1", "IIV-CL", "OMEGA(1,1)", "SIGMA(1,1)", "AddErr", "UNKNOWN")
  )

  expect_equal(transforms, c("LogNormal", "LogNormal", "LogNormal", NA))
  expect_equal(units, c("L/h", NA, NA, "ng/mL", "ng/mL", NA))
})

test_that("get_eta_labels uses diagonal omegas and sorts by row index", {
  theta1 <- ThetaComment(
    nonmem_name = "THETA1",
    name = "CL"
  )
  theta2 <- ThetaComment(
    nonmem_name = "THETA2",
    name = "V"
  )
  omega22 <- OmegaComment(
    nonmem_name = "OMEGA(2,2)",
    name = "IIV-V",
    associated_theta = "V"
  )
  omega21 <- OmegaComment(
    nonmem_name = "OMEGA(2,1)",
    name = "Corr-CL-V",
    associated_theta = c("CL", "V")
  )
  omega11 <- OmegaComment(
    nonmem_name = "OMEGA(1,1)",
    name = "IIV-CL",
    associated_theta = "CL"
  )

  info <- ModelComments(
    theta = list(THETA1 = theta1, THETA2 = theta2),
    omega = list(
      `OMEGA(2,2)` = omega22,
      `OMEGA(2,1)` = omega21,
      `OMEGA(1,1)` = omega11
    ),
    sigma = list()
  )

  expect_equal(
    get_eta_labels(info),
    c("ETA1//ETA-CL", "ETA2//ETA-V")
  )
})
