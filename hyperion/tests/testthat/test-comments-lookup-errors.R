test_that("add_parameter_to_lookup validates inputs", {
  lookup_path <- tempfile(fileext = ".toml")

  expect_error(
    add_parameter_to_lookup(lookup_path),
    "name is required"
  )

  expect_error(
    add_parameter_to_lookup(
      lookup_path,
      "TVCL",
      parameterization = "BAD"
    ),
    "Invalid parameterization"
  )

  expect_error(
    add_parameter_to_lookup(lookup_path, "TVCL"),
    "At least one of display"
  )

  add_parameter_to_lookup(lookup_path, "TVCL", display = "CL")
  expect_error(
    add_parameter_to_lookup(lookup_path, "TVCL", display = "CL"),
    "already exists"
  )
})

test_that("remove_parameter_from_lookup errors on missing file", {
  missing_lookup <- file.path(
    system.file("extdata", package = "hyperion"),
    "missing_lookup.toml"
  )
  expect_error(
    remove_parameter_from_lookup(missing_lookup, "TVCL"),
    "Lookup file not found"
  )
})
