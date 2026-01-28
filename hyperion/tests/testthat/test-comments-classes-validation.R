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

test_that("ModelComments renames duplicate omega names to name-associated_theta", {
  theta1 <- ThetaComment(nonmem_name = "THETA1", name = "CL")
  theta2 <- ThetaComment(nonmem_name = "THETA2", name = "V")
  theta3 <- ThetaComment(nonmem_name = "THETA3", name = "KA")

  # All three omegas have the same name "IIV" but different associated_theta
  omega1 <- OmegaComment(
    nonmem_name = "OMEGA(1,1)",
    name = "IIV",
    associated_theta = "CL"
  )
  omega2 <- OmegaComment(
    nonmem_name = "OMEGA(2,2)",
    name = "IIV",
    associated_theta = "V"
  )
  omega3 <- OmegaComment(
    nonmem_name = "OMEGA(3,3)",
    name = "IIV",
    associated_theta = "KA"
  )

  # Set up sources to test audit trail
  attr(omega1, "sources") <- list(name = "test.lst")
  attr(omega2, "sources") <- list(name = "test.lst")
  attr(omega3, "sources") <- list(name = "test.lst")

  info <- ModelComments(
    theta = list(THETA1 = theta1, THETA2 = theta2, THETA3 = theta3),
    omega = list(
      `OMEGA(1,1)` = omega1,
      `OMEGA(2,2)` = omega2,
      `OMEGA(3,3)` = omega3
    )
  )

  # All names should be renamed to include associated_theta
  expect_equal(info@omega[["OMEGA(1,1)"]]@name, "IIV-CL")
  expect_equal(info@omega[["OMEGA(2,2)"]]@name, "IIV-V")
  expect_equal(info@omega[["OMEGA(3,3)"]]@name, "IIV-KA")

  # Audit should show "renamed from" source
  audit <- audit_parameter_info(info)
  expect_true(all(grepl("renamed from", audit$omega$name)))
})

test_that("raw comment parsing renames duplicate omega names", {
  mod_files <- c(
    "run-duplicate-omega-names-with-theta.mod",
    "run-duplicate-omega-names-space.mod"
  )

  for (mod_file in mod_files) {
    mod_path <- testthat::test_path("testdata", "models", mod_file)
    mod <- read_model(mod_path)
    param_names <- get_model_parameter_names(mod)
    comments_data <- hyperion:::extract_comments(mod)
    comments <- hyperion:::parse_comments(
      param_names,
      comments_data$parsed,
      comments_data$raw,
      mod_path
    )

    theta_comments <- comments[grepl("^THETA", names(comments))]
    omega_comments <- comments[grepl("^OMEGA", names(comments))]

    info <- ModelComments(theta = theta_comments, omega = omega_comments)

    # All omega names should be unique after renaming
    omega_names <- vapply(
      info@omega,
      function(c) c@name,
      character(1)
    )
    expect_equal(
      length(omega_names),
      length(unique(omega_names)),
      info = paste("Failed for:", mod_file)
    )

    # Audit should show "renamed from" for all omega names
    audit <- audit_parameter_info(info)
    expect_true(
      all(grepl("renamed from", audit$omega$name)),
      info = paste("Failed for:", mod_file)
    )
  }
})
