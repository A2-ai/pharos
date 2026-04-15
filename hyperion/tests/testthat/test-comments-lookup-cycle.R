test_that("resolve_unit detects cycles in lookup references", {
  lookup <- list(
    A = list(unit = "B"),
    B = list(unit = "A")
  )

  on.exit(setTimeLimit(cpu = Inf, elapsed = Inf, transient = FALSE), add = TRUE)
  setTimeLimit(cpu = 0.5, elapsed = 0.5, transient = TRUE)

  expect_error(
    hyperion:::resolve_unit("A", lookup),
    "cycle"
  )
})
