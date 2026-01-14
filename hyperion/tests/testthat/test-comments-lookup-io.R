test_that("lookup file helpers list and remove parameters", {
  skip_if_not_installed("tomledit")

  lookup_path <- tempfile(fileext = ".toml")
  toml <- tomledit::toml()
  toml <- tomledit::insert_items(
    toml,
    TVCL = list(display = "CL"),
    TVV = list(display = "V")
  )
  tomledit::write_toml(toml, lookup_path)

  expect_equal(
    sort(list_lookup_parameters(lookup_path)),
    c("TVCL", "TVV")
  )

  expect_warning(
    remove_parameter_from_lookup(lookup_path, "MISSING"),
    "not found"
  )

  remove_parameter_from_lookup(lookup_path, "TVCL")
  expect_equal(list_lookup_parameters(lookup_path), "TVV")
})
