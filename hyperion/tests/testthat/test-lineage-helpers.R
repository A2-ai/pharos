test_that("get_model_ancestors returns ancestors in order", {
  tree <- structure(
    list(
      nodes = list(
        "run001.mod" = list(based_on = list(), description = "Base model"),
        "run002.mod" = list(
          based_on = list("run001.mod"),
          description = "Child"
        ),
        "run003.mod" = list(
          based_on = list("run002.mod"),
          description = "Grandchild"
        )
      )
    ),
    class = "hyperion_nonmem_tree"
  )

  # run001 has no ancestors

  expect_equal(get_model_ancestors(tree, "run001"), character(0))
  expect_equal(get_model_ancestors(tree, "run001.mod"), character(0))

  # run002's ancestor is run001
  expect_equal(get_model_ancestors(tree, "run002"), "run001")

  # run003's ancestors are run002, run001 (parent to root order)
  expect_equal(get_model_ancestors(tree, "run003"), c("run002", "run001"))
})

test_that("get_model_descendants returns all descendants", {
  tree <- structure(
    list(
      nodes = list(
        "run001.mod" = list(based_on = list(), description = "Base model"),
        "run002.mod" = list(
          based_on = list("run001.mod"),
          description = "Child 1"
        ),
        "run003.mod" = list(
          based_on = list("run001.mod"),
          description = "Child 2"
        ),
        "run004.mod" = list(
          based_on = list("run002.mod"),
          description = "Grandchild"
        )
      )
    ),
    class = "hyperion_nonmem_tree"
  )

  # run001 has three descendants
  descendants <- get_model_descendants(tree, "run001")
  expect_true(all(c("run002", "run003", "run004") %in% descendants))

  # run002 has one descendant
  expect_equal(get_model_descendants(tree, "run002"), "run004")

  # run003 has no descendants
  expect_equal(get_model_descendants(tree, "run003"), character(0))

  # run004 has no descendants

  expect_equal(get_model_descendants(tree, "run004"), character(0))
})

test_that("are_models_in_lineage detects ancestor-descendant relationships", {
  tree <- structure(
    list(
      nodes = list(
        "run001.mod" = list(based_on = list(), description = "Base model"),
        "run002.mod" = list(
          based_on = list("run001.mod"),
          description = "Child 1"
        ),
        "run003.mod" = list(
          based_on = list("run001.mod"),
          description = "Child 2"
        ),
        "run004.mod" = list(
          based_on = list("run002.mod"),
          description = "Grandchild"
        )
      )
    ),
    class = "hyperion_nonmem_tree"
  )

  # Direct parent-child

  expect_true(are_models_in_lineage(tree, "run001", "run002"))
  expect_true(are_models_in_lineage(tree, "run002", "run001"))

  # Grandparent-grandchild
  expect_true(are_models_in_lineage(tree, "run001", "run004"))
  expect_true(are_models_in_lineage(tree, "run004", "run001"))

  # Siblings are NOT in direct lineage

  expect_false(are_models_in_lineage(tree, "run002", "run003"))
  expect_false(are_models_in_lineage(tree, "run003", "run002"))

  # Cousins are NOT in direct lineage
  expect_false(are_models_in_lineage(tree, "run003", "run004"))
})

test_that("lineage functions handle .mod suffix correctly", {
  tree <- structure(
    list(
      nodes = list(
        "run001.mod" = list(based_on = list(), description = "Base"),
        "run002.mod" = list(
          based_on = list("run001.mod"),
          description = "Child"
        )
      )
    ),
    class = "hyperion_nonmem_tree"
  )

  # With and without .mod suffix should work
  expect_equal(get_model_ancestors(tree, "run002"), "run001")
  expect_equal(get_model_ancestors(tree, "run002.mod"), "run001")

  expect_true(are_models_in_lineage(tree, "run001", "run002"))
  expect_true(are_models_in_lineage(tree, "run001.mod", "run002.mod"))
  expect_true(are_models_in_lineage(tree, "run001", "run002.mod"))
})

test_that("lineage functions error on invalid input", {
  not_a_tree <- list(nodes = list())

  expect_error(get_model_ancestors(not_a_tree, "run001"))
  expect_error(get_model_descendants(not_a_tree, "run001"))
  expect_error(are_models_in_lineage(not_a_tree, "run001", "run002"))
})

test_that("get_model_ancestors errors on circular lineage", {
  tree <- structure(
    list(
      nodes = list(
        "run001.mod" = list(based_on = list("run002.mod")),
        "run002.mod" = list(based_on = list("run001.mod"))
      )
    ),
    class = "hyperion_nonmem_tree"
  )

  expect_error(get_model_ancestors(tree, "run001"))
})
