test_that("get_comment returns NULL for non-parameter names", {
  info <- ModelComments()
  expect_null(get_comment(info, "OTHER1"))
})

test_that("resolve_comment strips name suffixes", {
  theta1 <- ThetaComment(nonmem_name = "THETA1", name = "CL")
  info <- ModelComments(theta = list(THETA1 = theta1))

  expect_equal(
    get_parameter_transform(info, "THETA1 (CL)"),
    "Identity"
  )
})

test_that("get_theta_names rejects non-ModelComments input", {
  expect_error(
    get_theta_names(list()),
    "model_comments must be a ModelComments object"
  )
})

test_that("get_parameter_names returns empty data frame when no rows", {
  info <- ModelComments()
  result <- get_parameter_names(info)
  expect_equal(nrow(result), 0)
  expect_equal(names(result), c("name", "display"))
})
