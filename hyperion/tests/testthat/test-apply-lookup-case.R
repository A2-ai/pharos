test_that("apply_lookup_defaults matches names case-insensitively", {
  lookup_path <- tempfile(fileext = ".toml")
  toml <- tomledit::toml()
  toml <- tomledit::insert_items(
    toml,
    TVCL = list(display = "CL", unit = "L/h")
  )
  tomledit::write_toml(toml, lookup_path)

  comment <- ThetaComment(
    nonmem_name = "THETA1",
    name = "tvcl"
  )

  updated <- apply_lookup_defaults(comment, lookup_path)

  expect_equal(updated@display, "CL")
  expect_equal(updated@unit, "L/h")
})

test_that("apply_lookup_defaults resolves unit references", {
  lookup_path <- tempfile(fileext = ".toml")
  toml <- tomledit::toml()
  toml <- tomledit::insert_items(
    toml,
    VOLUME = list(unit = "L"),
    TIME = list(unit = "h"),
    TVCL = list(unit = "VOLUME/TIME")
  )
  tomledit::write_toml(toml, lookup_path)

  comment <- ThetaComment(
    nonmem_name = "THETA1",
    name = "TVCL"
  )

  updated <- apply_lookup_defaults(comment, lookup_path)

  expect_equal(updated@unit, "L/h")
})

test_that("apply_lookup_defaults applies units to sigma comments", {
  lookup_path <- tempfile(fileext = ".toml")
  toml <- tomledit::toml()
  toml <- tomledit::insert_items(
    toml,
    AddErr = list(unit = "ng/mL")
  )
  tomledit::write_toml(toml, lookup_path)

  comment <- SigmaComment(
    nonmem_name = "SIGMA(1,1)",
    name = "AddErr"
  )

  updated <- apply_lookup_defaults(comment, lookup_path)

  expect_equal(updated@unit, "ng/mL")
})
