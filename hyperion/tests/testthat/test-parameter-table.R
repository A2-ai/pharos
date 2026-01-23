test_that("parameter table: run001 basic spec", {
  model_dir <- system.file(
    "extdata",
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

  table_gt <- params |>
    apply_table_spec(spec, mod_info) |>
    add_summary_info(mod_sum) |>
    make_parameter_table()

  snapshot_gt(table_gt, "param-run001-basic-gt")

  table_ft <- params |>
    apply_table_spec(spec, mod_info) |>
    add_summary_info(mod_sum) |>
    make_parameter_table(output = "flextable")

  snapshot_flextable(table_ft, "param-run001-ft")
})

test_that("parameter table: run001 shows fixed", {
  model_dir <- system.file(
    "extdata",
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

  table_gt <- params |>
    apply_table_spec(spec, mod_info) |>
    add_summary_info(mod_sum) |>
    make_parameter_table()

  snapshot_gt(table_gt, "param-run001-fixed-gt")

  table_ft <- params |>
    apply_table_spec(spec, mod_info) |>
    add_summary_info(mod_sum) |>
    make_parameter_table(output = "flextable")

  snapshot_flextable(table_ft, "param-run001-fixed-ft")
})

test_that("parameter table: run002 shows empty fixed", {
  model_dir <- system.file(
    "extdata",
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

  table_gt <- params |>
    apply_table_spec(spec, mod_info) |>
    add_summary_info(mod_sum) |>
    make_parameter_table()

  snapshot_gt(table_gt, "param-run002-no-fixed-gt")

  table_ft <- params |>
    apply_table_spec(spec, mod_info) |>
    add_summary_info(mod_sum) |>
    make_parameter_table(output = "flextable")

  snapshot_flextable(table_ft, "param-run002-no-fixed-ft")

  spec@add_columns <- "fixed"
  table_gt <- params |>
    apply_table_spec(spec, mod_info) |>
    add_summary_info(mod_sum) |>
    make_parameter_table()

  snapshot_gt(table_gt, "param-run002-fixed-gt")

  spec@add_columns <- "fixed"
  table_ft <- params |>
    apply_table_spec(spec, mod_info) |>
    add_summary_info(mod_sum) |>
    make_parameter_table(output = "flextable")

  snapshot_flextable(table_ft, "param-run002-fixed-ft")
})

test_that("parameter table: run003 drop ci column", {
  model_dir <- system.file(
    "extdata",
    "models",
    "onecmt",
    "run003",
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
    drop_columns = "ci"
  )

  table_gt <- params |>
    apply_table_spec(spec, mod_info) |>
    add_summary_info(mod_sum) |>
    make_parameter_table()

  snapshot_gt(table_gt, "param-run003-drop-ci-gt")

  table_ft <- params |>
    apply_table_spec(spec, mod_info) |>
    add_summary_info(mod_sum) |>
    make_parameter_table(output = "flextable")

  snapshot_flextable(table_ft, "param-run003-drop-ci-ft")
})

test_that("parameter table: run003 drop ci_low column", {
  model_dir <- system.file(
    "extdata",
    "models",
    "onecmt",
    "run003",
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
    drop_columns = "ci_low"
  )

  table_gt <- params |>
    apply_table_spec(spec, mod_info) |>
    add_summary_info(mod_sum) |>
    make_parameter_table()

  snapshot_gt(table_gt, "param-run003-drop-ci_low-gt")

  table_ft <- params |>
    apply_table_spec(spec, mod_info) |>
    add_summary_info(mod_sum) |>
    make_parameter_table(output = "flextable")

  snapshot_flextable(table_ft, "param-run003-drop-ci_low-ft")
})

test_that("parameter table: run003 drop ci_high column", {
  model_dir <- system.file(
    "extdata",
    "models",
    "onecmt",
    "run003",
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
    drop_columns = "ci_high"
  )

  table_gt <- params |>
    apply_table_spec(spec, mod_info) |>
    add_summary_info(mod_sum) |>
    make_parameter_table()

  snapshot_gt(table_gt, "param-run003-drop-ci_high-gt")

  table_ft <- params |>
    apply_table_spec(spec, mod_info) |>
    add_summary_info(mod_sum) |>
    make_parameter_table(output = "flextable")

  snapshot_flextable(table_ft, "param-run003-drop-ci_high-ft")
})

test_that("parameter table: run003 summary footnote only", {
  model_dir <- system.file(
    "extdata",
    "models",
    "onecmt",
    "run003",
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
    drop_columns = "ci",
    footnote_order = "summary_info"
  )

  table_gt <- params |>
    apply_table_spec(spec, mod_info) |>
    add_summary_info(mod_sum) |>
    make_parameter_table()

  snapshot_gt(table_gt, "param-run003-summary-fn-gt")

  table_ft <- params |>
    apply_table_spec(spec, mod_info) |>
    add_summary_info(mod_sum) |>
    make_parameter_table(output = "flextable")

  snapshot_flextable(table_ft, "param-run003-summary-fn-ft")
})

test_that("parameter table: run003 drop footnotes", {
  model_dir <- system.file(
    "extdata",
    "models",
    "onecmt",
    "run003",
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
    drop_columns = "ci",
    footnote_order = NULL
  )

  table_gt <- params |>
    apply_table_spec(spec, mod_info) |>
    add_summary_info(mod_sum) |>
    make_parameter_table()

  snapshot_gt(table_gt, "param-run003-no-fn-gt")

  table_ft <- params |>
    apply_table_spec(spec, mod_info) |>
    add_summary_info(mod_sum) |>
    make_parameter_table(output = "flextable")

  snapshot_flextable(table_ft, "param-run003-no-fn-ft")
})

test_that("parameter table: run001 no spec", {
  model_dir <- system.file(
    "extdata",
    "models",
    "onecmt",
    "run001",
    package = "hyperion"
  )
  testthat::skip_if_not(nzchar(model_dir), "Test data directory not found")

  params <- get_parameters(model_dir)

  expect_error(
    make_parameter_table(params),
    "TableSpec not found. Run apply_table_spec"
  )
})
