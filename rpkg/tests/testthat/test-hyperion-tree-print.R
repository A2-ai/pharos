test_that("hyperion_nonmem_tree print works", {
  tree <- structure(
    list(
      nodes = list(
        "base.mod" = list(
          based_on = list(),
          description = "Base population PK model"
        ),
        "run001.mod" = list(
          based_on = list("base.mod"),
          description = "Run 1"
        ),
        "run002.mod" = list(
          based_on = list("run001.mod"),
          description = "Run 2 with covariate effects"
        )
      )
    ),
    class = "hyperion_nonmem_tree"
  )
  expect_snapshot(print(tree))
})
