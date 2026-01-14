test_that("TableSpec allows random_effect column", {
  expect_silent(TableSpec(columns = c("name", "random_effect", "estimate")))
})
