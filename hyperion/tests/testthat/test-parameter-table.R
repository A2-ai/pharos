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

  snapshot_gt(table_gt, "parameter-table-run001-basic")

  table_ft <- params |>
    apply_table_spec(spec, mod_info) |>
    add_summary_info(mod_sum) |>
    make_parameter_table(output = "flextable")

  snapshot_flextable(table_ft, "parameter-table-run001-flex")
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

  snapshot_gt(table_gt, "parameter-table-run001-fixed")

  table_ft <- params |>
    apply_table_spec(spec, mod_info) |>
    add_summary_info(mod_sum) |>
    make_parameter_table(output = "flextable")

  snapshot_flextable(table_ft, "parameter-table-run001-fixed-flex")
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

  snapshot_gt(table_gt, "parameter-table-run002-no-fixed")

  table_ft <- params |>
    apply_table_spec(spec, mod_info) |>
    add_summary_info(mod_sum) |>
    make_parameter_table(output = "flextable")

  snapshot_flextable(table_ft, "parameter-table-run002-no-fixed-flex")

  spec@add_columns <- "fixed"
  table_gt <- params |>
    apply_table_spec(spec, mod_info) |>
    add_summary_info(mod_sum) |>
    make_parameter_table()

  snapshot_gt(table_gt, "parameter-table-run002-fixed")

  spec@add_columns <- "fixed"
  table_ft <- params |>
    apply_table_spec(spec, mod_info) |>
    add_summary_info(mod_sum) |>
    make_parameter_table(output = "flextable")

  snapshot_flextable(table_ft, "parameter-table-run002-fixed-flex")
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

  snapshot_gt(table_gt, "parameter-table-run003-drop-ci")

  table_ft <- params |>
    apply_table_spec(spec, mod_info) |>
    add_summary_info(mod_sum) |>
    make_parameter_table(output = "flextable")

  snapshot_flextable(table_ft, "parameter-table-run003-drop-ci-flex")
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

  snapshot_gt(table_gt, "parameter-table-run003-drop-ci_low")

  table_ft <- params |>
    apply_table_spec(spec, mod_info) |>
    add_summary_info(mod_sum) |>
    make_parameter_table(output = "flextable")

  snapshot_flextable(table_ft, "parameter-table-run003-drop-ci_low-flex")
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

  snapshot_gt(table_gt, "parameter-table-run003-drop-ci_high")

  table_ft <- params |>
    apply_table_spec(spec, mod_info) |>
    add_summary_info(mod_sum) |>
    make_parameter_table(output = "flextable")

  snapshot_flextable(table_ft, "parameter-table-run003-drop-ci_high-flex")
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

  snapshot_gt(table_gt, "parameter-table-run003-summary-fn")

  table_ft <- params |>
    apply_table_spec(spec, mod_info) |>
    add_summary_info(mod_sum) |>
    make_parameter_table(output = "flextable")

  snapshot_flextable(table_ft, "parameter-table-run003-summary-fn-flex")
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

  snapshot_gt(table_gt, "parameter-table-run003-no-fn")

  table_ft <- params |>
    apply_table_spec(spec, mod_info) |>
    add_summary_info(mod_sum) |>
    make_parameter_table(output = "flextable")

  snapshot_flextable(table_ft, "parameter-table-run003-no-fn-flex")
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
