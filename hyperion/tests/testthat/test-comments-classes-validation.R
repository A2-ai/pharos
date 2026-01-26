test_that("ModelComments validates duplicate theta names", {
  theta1 <- ThetaComment(nonmem_name = "THETA1", name = "CL")
  theta2 <- ThetaComment(nonmem_name = "THETA2", name = "CL")

  expect_error(
    ModelComments(theta = list(THETA1 = theta1, THETA2 = theta2)),
    "Duplicate names in theta"
  )
})

test_that("ModelComments validates omega associated_theta existence", {
  theta1 <- ThetaComment(nonmem_name = "THETA1", name = "CL")
  omega11 <- OmegaComment(
    nonmem_name = "OMEGA(1,1)",
    name = "IIV",
    associated_theta = "V"
  )

  expect_error(
    ModelComments(
      theta = list(THETA1 = theta1),
      omega = list(`OMEGA(1,1)` = omega11)
    ),
    "associated_theta"
  )
})

test_that("ModelComments enforces comment class types", {
  theta1 <- ThetaComment(nonmem_name = "THETA1", name = "CL")

  expect_error(
    ModelComments(theta = list(THETA1 = theta1, THETA2 = "bad")),
    "must be a ThetaComment object"
  )
})

test_that("ModelComments allows unnamed omega duplicates from model files", {
  mod_path <- testthat::test_path(
    "testdata",
    "models",
    "run-duplicate-omega-names.mod"
  )
  mod <- read_model(mod_path)
  param_names <- get_model_parameter_names(mod)
  comments_data <- hyperion:::extract_comments(mod)
  comments <- hyperion:::parse_comments(
    param_names,
    comments_data$parsed,
    comments_data$raw,
    mod_path
  )
  omega_comments <- comments[grepl("^OMEGA", names(comments))]

  expect_no_error(ModelComments(omega = omega_comments))
})
