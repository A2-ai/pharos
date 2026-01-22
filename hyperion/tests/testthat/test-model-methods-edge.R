test_that("format_ignore_condition handles marker and unknowns", {
  expect_equal(
    format_ignore_condition(list(Marker = "@")),
    "@"
  )

  expect_equal(
    format_ignore_condition(list()),
    "Unknown"
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
