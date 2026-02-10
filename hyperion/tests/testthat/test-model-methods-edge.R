test_that("format_ignore_condition handles marker and unknowns", {
  expect_equal(
    format_ignore_condition(list(Marker = "@")),
    "@"
  )

  expect_equal(
    format_ignore_condition(list()),
    NA_character_
  )
})

test_that("format_ignore_condition falls back for unknown operators", {
  ignore_obj <- list(
    ValueFilter = list(
      field = "AN01FL",
      op = "Between",
      value = "0"
    )
  )

  expect_equal(format_ignore_condition(ignore_obj), "AN01FL.Between.0")
})

test_that("get_theta_parameter_data returns NULL with no parameters", {
  x <- list(theta_parameters = list())
  expect_null(get_theta_parameter_data(
    x,
    digits = NULL,
    theta_names = character()
  ))
})

test_that("get_random_effect_parameter_data errors on missing BlockSame reference", {
  blocks <- list(
    list(structure = list(BlockSame = list(size = 2)))
  )

  expect_error(
    get_random_effect_parameter_data(
      blocks,
      digits = NULL,
      param_names = c("OMEGA(1,1)", "OMEGA(2,1)")
    ),
    "BlockSame found but no previous Block structure"
  )
})

test_that("get_random_effect_parameter_data handles BlockSame copying", {
  blocks <- list(
    list(
      structure = list(Block = list(size = 2)),
      parametrization = "LogNormal",
      parameters = list(
        list(
          initial_value = 0.1,
          lower_bound = 0,
          upper_bound = 1,
          is_fixed = FALSE,
          comment = "A"
        ),
        list(
          initial_value = 0.2,
          lower_bound = 0,
          upper_bound = 1,
          is_fixed = TRUE,
          comment = "B"
        )
      )
    ),
    list(structure = list(BlockSame = list(size = 2)))
  )

  result <- get_random_effect_parameter_data(
    blocks,
    digits = NULL,
    param_names = c("OMEGA(1,1)", "OMEGA(2,1)", "OMEGA(2,2)", "OMEGA(3,2)")
  )

  expect_equal(nrow(result), 4)
  expect_equal(result$Parameter[3], "OMEGA(2,2)")
  expect_equal(result$Fixed[4], "Yes")
})

test_that("summary.hyperion_nonmem_model validates n_iterations", {
  mod <- structure(list(), class = "hyperion_nonmem_model")

  expect_error(
    summary(mod, n_iterations = 0),
    "`n_iterations` must be a single positive integer"
  )
  expect_error(
    summary(mod, n_iterations = -1),
    "`n_iterations` must be a single positive integer"
  )
  expect_error(
    summary(mod, n_iterations = 1.5),
    "`n_iterations` must be a single positive integer"
  )
  expect_error(
    summary(mod, n_iterations = "foo"),
    "`n_iterations` must be a single positive integer"
  )
})

test_that("build_running_summary limits iteration and gradient rows", {
  tmp_dir <- withr::local_tempdir()
  run_dir <- file.path(tmp_dir, "run004")
  dir.create(run_dir)
  file.create(file.path(run_dir, "run004.ext"))
  file.create(file.path(run_dir, "run004.grd"))

  mod_path <- file.path(tmp_dir, "run004.mod")
  object <- structure(list(), model_source = mod_path)

  ext_data <- data.frame(iter = 1:5, stringsAsFactors = FALSE)
  grd_data <- data.frame(grad = 11:15, stringsAsFactors = FALSE)

  testthat::local_mocked_bindings(
    get_model_name = function(object) "run004",
    from_config_relative = function(path) path,
    read_ext_file = function(...) ext_data,
    get_gradients = function(...) grd_data
  )

  result <- build_running_summary(object, n_iterations = 2)

  expect_equal(result$run_status, "running")
  expect_equal(result$iterations$iter, c(4L, 5L))
  expect_equal(result$gradients$grad, c(14L, 15L))
})

test_that("build_running_summary does not call Rust when ext/grd files missing", {
  tmp_dir <- withr::local_tempdir()
  mod_path <- file.path(tmp_dir, "run004.mod")
  object <- structure(list(), model_source = mod_path)

  testthat::local_mocked_bindings(
    get_model_name = function(object) "run004",
    from_config_relative = function(path) path,
    read_ext_file = function(...) stop("should not be called"),
    get_gradients = function(...) stop("should not be called")
  )

  result <- build_running_summary(object, n_iterations = 5)

  expect_equal(result$run_status, "running")
  expect_null(result$iterations)
  expect_null(result$gradients)
})
