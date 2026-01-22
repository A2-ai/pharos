test_that("TableSpec validates key properties", {
  expect_error(
    TableSpec(display_transforms = list(foo = "all")),
    "@display_transforms names must be"
  )

  expect_error(
    TableSpec(display_transforms = list(theta = "bad")),
    "@display_transforms values must be"
  )

  expect_error(
    TableSpec(columns = c("name", "bogus")),
    "@columns must be"
  )

  expect_error(
    TableSpec(ci_level = 1.5),
    "@ci_level must be between"
  )

  expect_error(
    TableSpec(n_sigfig = 2.5),
    "@n_sigfig must be a positive whole number"
  )

  expect_error(
    TableSpec(name_source = "other"),
    "@name_source must be"
  )
})
