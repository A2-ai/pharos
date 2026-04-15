test_that("get_parameter_unit requires kind when names collide", {
  theta1 <- ThetaComment(
    nonmem_name = "THETA1",
    name = "CL",
    unit = "L/h"
  )
  omega11 <- OmegaComment(
    nonmem_name = "OMEGA(1,1)",
    name = "CL"
  )

  info <- ModelComments(
    theta = list(THETA1 = theta1),
    omega = list(`OMEGA(1,1)` = omega11),
    sigma = list()
  )

  expect_error(
    get_parameter_unit(info, "CL"),
    "Ambiguous parameter name"
  )

  expect_equal(get_parameter_unit(info, "CL", kind = "THETA"), "L/h")
})
