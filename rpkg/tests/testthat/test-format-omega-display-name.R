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

  # Theta already in name via slash - no duplication
  expect_equal(
    format_omega_display_name("IIV-CL", "CL/F"),
    "IIV-CL"
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

  # With custom labels that include spaces - label already present
  expect_equal(
    format_omega_display_name(
      "IIV CL/F Scaling",
      "CLF",
      c("CLF" = "CL/F Scaling")
    ),
    "IIV CL/F Scaling"
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

test_that("format_omega_display_name matches theta roots after stripping TV/ETA prefix", {
  # CL in name matches TVCL theta (TVCL -> CL)
  expect_equal(
    format_omega_display_name("IIV-CL", "TVCL"),
    "IIV-CL"
  )

  # Vc in name matches TVVC theta (TVVC -> VC, case-insensitive)
  expect_equal(
    format_omega_display_name("IIV-Vc", "TVVC"),
    "IIV-Vc"
  )

  # KA in name matches TVKA theta (TVKA -> KA)
  expect_equal(
    format_omega_display_name("IIV-KA", "TVKA"),
    "IIV-KA"
  )

  # Multiple TV-prefixed thetas
  expect_equal(
    format_omega_display_name("COV-CL-V", c("TVCL", "TVV")),
    "COV-CL-V"
  )

  # Partial match - only CL present, V missing (TVV -> V, not in name)
  expect_equal(
    format_omega_display_name("IIV-CL", c("TVCL", "TVV")),
    "IIV-CL TVV"
  )

  # ETA prefix also stripped
  expect_equal(
    format_omega_display_name("IIV-CL", "ETACL"),
    "IIV-CL"
  )
})


test_that("renaming work for off-diags", {
  model_dir <- system.file("extdata", "models", "onecmt", package = "hyperion")

  mod <- read_model(file.path(model_dir, "run003.mod"))
  info <- get_model_parameter_info(mod)

  display_name <- format_omega_display_name(
    info@omega$`OMEGA(2,1)`@name,
    info@omega$`OMEGA(2,1)`@associated_theta
  )
  expect_equal(display_name, "OM1,2 TVCL, TVV")
  expect_equal(info@omega$`OMEGA(2,1)`@name, "OM1,2")
  expect_equal(info@omega$`OMEGA(2,1)`@associated_theta, c("TVCL", "TVV"))
})
