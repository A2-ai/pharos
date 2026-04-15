test_that("format_ignore_condition formats value filters", {
  ignore_obj <- list(
    ValueFilter = list(
      field = "AN01FL",
      op = "Equal",
      value = "0"
    )
  )

  expect_equal(format_ignore_condition(ignore_obj), "AN01FL.EQ.0")
})
