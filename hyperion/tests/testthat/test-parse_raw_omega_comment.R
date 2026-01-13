test_that("parse omega comments extracts name and associated_theta separately", {
  # Name is prefix only, associated_theta stored separately
  om_comment <- parse_raw_omega_comment("OMEGA(2,1)", NULL, "OM2,1 CL-VC")
  expect_equal(om_comment@nonmem_name, "OMEGA(2,1)")
  expect_equal(om_comment@name, "OM2,1")
  expect_equal(om_comment@parameterization, NULL)
  expect_equal(om_comment@associated_theta, c("CL", "VC"))

  om_comment <- parse_raw_omega_comment("OMEGA(2,1)", NULL, "OM2,1 CL-VC ;log")
  expect_equal(om_comment@nonmem_name, "OMEGA(2,1)")
  expect_equal(om_comment@name, "OM2,1")
  expect_equal(om_comment@parameterization, "LogNormal")
  expect_equal(om_comment@associated_theta, c("CL", "VC"))
})

test_that("omega name is prefix only, associated_theta stored separately", {
  # Already hyphenated - extracts prefix and theta
  result <- extract_raw_omega_parts("IIV-CL")
  expect_equal(result$name, "IIV")
  expect_equal(result$associated_theta, "CL")

  # Space between prefix and theta
  result <- extract_raw_omega_parts("IIV CL")
  expect_equal(result$name, "IIV")
  expect_equal(result$associated_theta, "CL")

  # Linking word "on" - skipped
  result <- extract_raw_omega_parts("IIV on CL")
  expect_equal(result$name, "IIV")
  expect_equal(result$associated_theta, "CL")

  # Different prefix
  result <- extract_raw_omega_parts("OM1 CL")
  expect_equal(result$name, "OM1")
  expect_equal(result$associated_theta, "CL")

  # Another linking word
  result <- extract_raw_omega_parts("eta on V")
  expect_equal(result$name, "eta")
  expect_equal(result$associated_theta, "V")

  # Correlation with hyphenated thetas
  result <- extract_raw_omega_parts("Corr CL-V")
  expect_equal(result$name, "Corr")
  expect_equal(result$associated_theta, c("CL", "V"))
})

test_that("associated_theta splits unless matches known theta", {
  # Without context - splits on separators
  result <- extract_raw_omega_parts("IIV on CL/F")
  expect_equal(result$name, "IIV")
  expect_equal(result$associated_theta, c("CL", "F"))

  result <- extract_raw_omega_parts("Corr CL/V")
  expect_equal(result$name, "Corr")
  expect_equal(result$associated_theta, c("CL", "V"))

  # With known_thetas context - preserves known names
  result <- extract_raw_omega_parts("IIV on CL/F", known_thetas = c("CL/F", "V"))
  expect_equal(result$name, "IIV")
  expect_equal(result$associated_theta, "CL/F")

  # Case-insensitive match, preserve original case
  result <- extract_raw_omega_parts("IIV on cl/f", known_thetas = c("CL/F"))
  expect_equal(result$name, "IIV")
  expect_equal(result$associated_theta, "cl/f")

  # With context but no match - still splits
  result <- extract_raw_omega_parts("Corr CL/V", known_thetas = c("CL/F", "KA"))
  expect_equal(result$name, "Corr")
  expect_equal(result$associated_theta, c("CL", "V"))
})

test_that("linking words are skipped", {
  result <- extract_raw_omega_parts("IIV on CL")
  expect_equal(result$associated_theta, "CL")

  result <- extract_raw_omega_parts("IIV for V")
  expect_equal(result$associated_theta, "V")

  result <- extract_raw_omega_parts("eta of KA")
  expect_equal(result$associated_theta, "KA")
})

test_that("split_theta_reference respects known thetas", {
  # No known thetas - splits
  expect_equal(split_theta_reference("CL/F"), c("CL", "F"))
  expect_equal(split_theta_reference("CL-V"), c("CL", "V"))
  expect_equal(split_theta_reference("CL"), "CL")

  # Known theta - keeps as-is
  expect_equal(split_theta_reference("CL/F", c("CL/F", "V")), "CL/F")

  # Case-insensitive match
  expect_equal(split_theta_reference("cl/f", c("CL/F")), "cl/f")

  # No match in known - splits
  expect_equal(split_theta_reference("CL/V", c("CL/F", "KA")), c("CL", "V"))
})
