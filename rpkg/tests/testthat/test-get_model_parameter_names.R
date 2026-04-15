test_that("get_model_parameter_names works for typed comments", {
  mod_dir <- system.file("extdata", "models", "onecmt", package = "hyperion")

  # run1 has incorrect type1 comments so nothing is parsed
  run1 <- read_model(file.path(mod_dir, "run001.mod"))
	expect_equal(get_comment_type(), "type1")

  n1 <- get_model_parameter_names(run1)
  expect_equal(all(n1 == ""), TRUE)

  # run2 has correct type1 comments for THETA/OMEGA
  # so parameter name should have non-empty values.
  # Sigma is incorrect and will be empty
  run2 <- read_model(file.path(mod_dir, "run002.mod"))
  n2 <- get_model_parameter_names(run2)
  expect_equal(any(n2 != ""), TRUE)
  expect_equal(n2$THETA1, "TVCL")
  expect_equal(n2$THETA2, "TVV")
  expect_equal(n2$`SIGMA(1,1)`, "")
  expect_equal(n2$`SIGMA(2,2)`, "")
})

test_that("get_parameter_names works for all comments", {
  mod_dir <- system.file("extdata", "models", "onecmt", package = "hyperion")

  # run1 has incorrect type1 comments so nothing is parsed
  run1 <- read_model(file.path(mod_dir, "run001.mod"))
  n1 <- get_parameter_names(run1)
  expect_equal(n1["THETA1", "name"], "TVCL")
  expect_equal(n1["THETA2", "name"], "TVV")

  # run2 has correct type1 comments for THETA/OMEGA
  # so parameter name should have non-empty values.
  # Sigma is incorrect and will be empty, OMEGA is
	# processed differently ((theta) vs , theta)
  run2 <- read_model(file.path(mod_dir, "run002.mod"))
  n2 <- get_parameter_names(run2)
  n2_mp <- get_model_parameter_names(run2)
  for (p in rownames(n2)) {
    if (grepl("^THETA", p)) {
      expect_equal(n2[p, "name"], n2_mp[[p]])
    }
  }
})
