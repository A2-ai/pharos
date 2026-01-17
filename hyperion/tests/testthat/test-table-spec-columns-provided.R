test_that("TableSpec columns_provided tracks explicit columns", {
  spec <- TableSpec()
  expect_false(isTRUE(spec@.columns_provided))

  spec@columns <- c("symbol", "fixed")
  expect_true(isTRUE(spec@.columns_provided))
})
