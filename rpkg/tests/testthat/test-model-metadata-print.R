test_that("hyperion_model_metadata print works (populated)", {
  meta <- structure(
    list(
      description = "Base population PK model",
      tags = c("pk", "base"),
      based_on = c("run000.mod")
    ),
    class = "hyperion_model_metadata"
  )

  expect_snapshot(print(meta))
})

test_that("hyperion_model_metadata print works (minimal)", {
  meta <- structure(
    list(
      description = "Base model",
      tags = character(0),
      based_on = character(0)
    ),
    class = "hyperion_model_metadata"
  )

  expect_snapshot(print(meta))
})
