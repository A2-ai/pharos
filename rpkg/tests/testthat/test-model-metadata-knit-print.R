test_that("hyperion_model_metadata knit_print works (populated)", {
  meta <- structure(
    list(
      description = "Base population PK model",
      tags = c("pk", "base"),
      based_on = c("run000.mod")
    ),
    class = "hyperion_model_metadata"
  )

  snapshot_knit_html(meta, "model-metadata-knit-populated")
})

test_that("hyperion_model_metadata knit_print works (minimal)", {
  meta <- structure(
    list(
      description = "Base model",
      tags = character(0),
      based_on = character(0)
    ),
    class = "hyperion_model_metadata"
  )

  snapshot_knit_html(meta, "model-metadata-knit-minimal")
})
