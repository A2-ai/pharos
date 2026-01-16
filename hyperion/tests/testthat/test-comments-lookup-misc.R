test_that("load_lookup_toml errors on missing file", {
  missing_lookup <- file.path(
    system.file("extdata", package = "hyperion"),
    "missing_lookup.toml"
  )
  expect_error(
    hyperion:::load_lookup_toml(missing_lookup),
    "Lookup file not found"
  )
})

test_that("resolve_unit expands nested references", {
  lookup <- list(
    VOLUME = list(unit = "L"),
    TIME = list(unit = "h"),
    TVCL = list(unit = "VOLUME/TIME")
  )

  expect_equal(hyperion:::resolve_unit("TVCL", lookup), "L/h")
  expect_equal(hyperion:::resolve_unit("VOLUME/TIME", lookup), "L/h")
})

test_that("apply_lookup_defaults ignores parameterization set to none", {
  skip_if_not_installed("tomledit")

  lookup_path <- tempfile(fileext = ".toml")
  toml <- tomledit::toml()
  toml <- tomledit::insert_items(
    toml,
    TVCL = list(parameterization = "none")
  )
  tomledit::write_toml(toml, lookup_path)

  comment <- ThetaComment(nonmem_name = "THETA1", name = "TVCL")
  updated <- apply_lookup_defaults(comment, lookup_path)

  expect_null(updated@parameterization)
})
