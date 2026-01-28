test_that("format_omega_display_name avoids duplicate theta info", {
  # Theta already in name via hyphen - no duplication
  expect_equal(
    format_omega_display_name("IIV-CL", "CL"),
    "IIV-CL"
  )

  # Theta already in name via slash - no duplication
  expect_equal(
    format_omega_display_name("IIV-CL/F", "CL/F"),
    "IIV-CL/F"
  )

  # Theta not in name - appends it
  expect_equal(
    format_omega_display_name("IIV", "CL"),
    "IIV CL"
  )

  # Multiple thetas, none present
  expect_equal(
    format_omega_display_name("COV", c("CL", "V")),
    "COV CL, V"
  )

  # Multiple thetas, some present
  expect_equal(
    format_omega_display_name("IIV-CL", c("CL", "V")),
    "IIV-CL V"
  )

  # With custom labels - label already present
  expect_equal(
    format_omega_display_name("IIV-Clearance", "CL", c(CL = "Clearance")),
    "IIV-Clearance"
  )

  # With custom labels - appends label not name
  expect_equal(
    format_omega_display_name("IIV", "CL", c(CL = "Clearance")),
    "IIV Clearance"
  )

  # NULL associated_theta returns name unchanged
  expect_equal(
    format_omega_display_name("IIV", NULL),
    "IIV"
  )

  # Empty associated_theta returns name unchanged
  expect_equal(
    format_omega_display_name("IIV", character(0)),
    "IIV"
  )
})
