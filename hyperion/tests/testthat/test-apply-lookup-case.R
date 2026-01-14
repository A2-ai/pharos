test_that("apply_lookup_defaults matches names case-insensitively", {
  skip_if_not_installed("tomledit")

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
  skip_if_not_installed("tomledit")

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
