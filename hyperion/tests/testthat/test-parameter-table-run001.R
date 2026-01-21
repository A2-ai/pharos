test_that("parameter table: run001 basic spec", {
  model_dir <- system.file("extdata",
    "models",
    "onecmt",
    "run001",
    package = "hyperion"
  )
  testthat::skip_if_not(nzchar(model_dir), "Test data directory not found")

  params <- get_parameters(model_dir)
  mod_info <- get_model_parameter_info(model_dir)
  mod_sum <- get_model_summary(model_dir)

  spec <- TableSpec(
    display_transforms = list(omega = "cv"),
    name_source = "display",
    title = "Model Parameters",
    sections = section_rules(
      kind == "THETA" ~ "Structural model parameters",
      kind == "OMEGA" & diagonal ~ "Interindividual variance parameters",
      kind == "OMEGA" & !diagonal ~ "Interindividual covariance parameters",
      kind == "SIGMA" ~ "Residual error",
      TRUE ~ "Other"
    )
  )

  table <- params |>
    apply_table_spec(spec, mod_info) |>
    add_summary_info(mod_sum) |>
    make_parameter_table()

  snapshot_gt(table, "parameter-table-run001-basic")
})

test_that("parameter table: run001 shows fixed", {
  model_dir <- system.file("extdata",
    "models",
    "onecmt",
    "run001",
    package = "hyperion"
  )
  testthat::skip_if_not(nzchar(model_dir), "Test data directory not found")

  params <- get_parameters(model_dir)
  mod_info <- get_model_parameter_info(model_dir)
  mod_sum <- get_model_summary(model_dir)

  spec <- TableSpec(
    display_transforms = list(omega = "cv"),
    name_source = "display",
    title = "Model Parameters",
    sections = section_rules(
      kind == "THETA" ~ "Structural model parameters",
      kind == "OMEGA" & diagonal ~ "Interindividual variance parameters",
      kind == "OMEGA" & !diagonal ~ "Interindividual covariance parameters",
      kind == "SIGMA" ~ "Residual error",
      TRUE ~ "Other"
    ),
    add_columns = "fixed"
  )

  table <- params |>
    apply_table_spec(spec, mod_info) |>
    add_summary_info(mod_sum) |>
    make_parameter_table()

  snapshot_gt(table, "parameter-table-run001-fixed")
})

test_that("parameter table: run002 shows empty fixed", {
  model_dir <- system.file("extdata",
    "models",
    "onecmt",
    "run002",
    package = "hyperion"
  )
  testthat::skip_if_not(nzchar(model_dir), "Test data directory not found")

  params <- get_parameters(model_dir)
  mod_info <- get_model_parameter_info(model_dir)
  mod_sum <- get_model_summary(model_dir)

  spec <- TableSpec(
    display_transforms = list(omega = "cv"),
    name_source = "display",
    title = "Model Parameters",
    sections = section_rules(
      kind == "THETA" ~ "Structural model parameters",
      kind == "OMEGA" & diagonal ~ "Interindividual variance parameters",
      kind == "OMEGA" & !diagonal ~ "Interindividual covariance parameters",
      kind == "SIGMA" ~ "Residual error",
      TRUE ~ "Other"
    )
  )

  table <- params |>
    apply_table_spec(spec, mod_info) |>
    add_summary_info(mod_sum) |>
    make_parameter_table()

  snapshot_gt(table, "parameter-table-run002-no-fixed")

  spec@add_columns <- "fixed"
  table <- params |>
    apply_table_spec(spec, mod_info) |>
    add_summary_info(mod_sum) |>
    make_parameter_table()

  snapshot_gt(table, "parameter-table-run002-fixed")
})
