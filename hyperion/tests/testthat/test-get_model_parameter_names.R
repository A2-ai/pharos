test_that("hyperion.nonmem-model print works", {
  mod_dir <- testthat::test_path("testdata", "models", "onecmt")

  # run1 has incorrect type1 comments so nothing is parsed
  run1 <- read_model(file.path(mod_dir, "run001.mod"))
  n1 <- get_model_parameter_names(run1)
  expect_equal(all(n1 == ""), TRUE)

  # run2 has correct type1 comments so parameter name
  # should have non-empty values.
  run2 <- read_model(file.path(mod_dir, "run002.mod"))
  n2 <- get_model_parameter_names(run2)
  expect_equal(any(n2 != ""), TRUE)
  expect_equal(n2$THETA1, "TVCL")
  expect_equal(n2$THETA2, "TVV")
})
