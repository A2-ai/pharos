test_that("format_omega_display_name does not duplicate associated theta names", {
  # Theta partially in name - only appends missing theta
  expect_equal(
    format_omega_display_name("Corr-CL", c("CL", "V")),
    "Corr-CL V"
  )

  # Both thetas already in name - no duplication
  expect_equal(
    format_omega_display_name("Corr-CL-V", c("CL", "V")),
    "Corr-CL-V"
  )
})

test_that("format_omega_display_name appends associated theta when not present", {
  # Single theta not in name
  expect_equal(
    format_omega_display_name("IIV", "TVCL"),
    "IIV TVCL"
  )

  # Multiple thetas not in name
  expect_equal(
    format_omega_display_name("IIV", c("TVCL", "TVV")),
    "IIV TVCL, TVV"
  )
})
