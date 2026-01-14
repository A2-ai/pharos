test_that("TableSpec allows stderr and diagonal columns", {
  expect_silent(
    TableSpec(columns = c("name", "stderr", "diagonal", "estimate"))
  )
})
