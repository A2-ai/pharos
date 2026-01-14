snapshot_gt_png <- function(table, name) {
  skip_if_not_installed("gt")

  path <- file.path(tempdir(), paste0(name, ".png"))
  gt::gtsave(table, filename = path)

  expect_snapshot_file(path)
}

test_that("parameter tables match vignette snapshots", {
  skip_if_not_installed("gt")

  model_dir <- file.path("testdata", "models", "onecmt")
  model_run <- "run003"
  lookup_path <- normalizePath(
    testthat::test_path("testdata", "lookup.toml")
  )

  spec <- TableSpec(
    display_transforms = list(omega = c("cv")),
    sections = section_rules(
      kind == "THETA" ~ "Structural model parameters",
      kind == "OMEGA" & diagonal ~ "Interindividual variance parameters",
      kind == "OMEGA" & !diagonal ~ "Interindividual covariance parameters",
      kind == "SIGMA" ~ "Residual error",
      TRUE ~ "Other"
    ),
    name_source = "display",
    drop_columns = "rse",
    title = paste(model_run, "Parameters")
  )

  model_path <- file.path(model_dir, model_run)
  info <- get_model_parameter_info(model_path, lookup_path)
  info@sigma$`SIGMA(1,1)`@parameterization <- "Proportional"

  mod_sum <- get_model_summary(file.path(model_dir, model_run))

  table <- get_parameters(file.path(model_dir, model_run)) |>
    apply_table_spec(info, spec) |>
    add_summary_info(mod_sum) |>
    make_parameter_table()
  snapshot_gt_png(table, "parameter-table-base")

  spec@name_source <- "display"
  info@sigma$`SIGMA(1,1)`@display <- "Proportional Error"
  info@sigma$`SIGMA(2,2)`@display <- "Additive Error"

  table <- get_parameters(file.path(model_dir, model_run)) |>
    apply_table_spec(info, spec) |>
    add_summary_info(mod_sum) |>
    make_parameter_table()
  snapshot_gt_png(table, "parameter-table-display")

  spec@name_source <- "nonmem_name"
  table <- get_parameters(file.path(model_dir, model_run)) |>
    apply_table_spec(info, spec) |>
    add_summary_info(mod_sum) |>
    make_parameter_table()
  snapshot_gt_png(table, "parameter-table-nonmem-name")

  spec@name_source <- "display"
  spec@show_description <- TRUE
  table <- get_parameters(file.path(model_dir, model_run)) |>
    apply_table_spec(info, spec) |>
    add_summary_info(mod_sum) |>
    make_parameter_table()
  snapshot_gt_png(table, "parameter-table-description")

  spec@show_description <- FALSE
  spec@drop_columns <- "unit"
  table <- get_parameters(file.path(model_dir, model_run)) |>
    apply_table_spec(info, spec) |>
    add_summary_info(mod_sum) |>
    make_parameter_table()
  snapshot_gt_png(table, "parameter-table-drop-unit")

  spec@drop_columns <- c("unit", "shrinkage")
  table <- get_parameters(file.path(model_dir, model_run)) |>
    apply_table_spec(info, spec) |>
    add_summary_info(mod_sum) |>
    make_parameter_table()
  snapshot_gt_png(table, "parameter-table-drop-unit-shrinkage")

  sp_spec <- TableSpec(
    sections = section_rules(
      kind == "THETA" ~ "Structural model parameters",
      TRUE ~ "Other"
    ),
    row_filter = filter_rules(
      kind == "THETA"
    ),
    drop_columns = "shrinkage"
  )
  table <- get_parameters(file.path(model_dir, model_run)) |>
    apply_table_spec(info, sp_spec) |>
    make_parameter_table()
  snapshot_gt_png(table, "parameter-table-structural-only")

  re_spec <- TableSpec(
    sections = section_rules(
      kind == "OMEGA" ~ "Random Effect Parameters",
      kind == "SIGMA" ~ "Residual Error",
      TRUE ~ "Other"
    ),
    row_filter = filter_rules(
      kind != "THETA"
    ),
    drop_columns = "unit"
  )
  table <- get_parameters(file.path(model_dir, model_run)) |>
    apply_table_spec(info, re_spec) |>
    make_parameter_table()
  snapshot_gt_png(table, "parameter-table-random-effects")

  spec <- TableSpec(
    display_transforms = list(omega = c("cv")),
    sections = section_rules(
      kind == "THETA" ~ "Structural model parameters",
      kind == "OMEGA" & diagonal ~ "Interindividual variance parameters",
      kind == "OMEGA" & !diagonal ~ "Interindividual covariance parameters",
      kind == "SIGMA" ~ "Residual variance",
      TRUE ~ "Other"
    ),
    ci_level = 0.7,
    n_sigfig = 3
  )
  mod_sum <- get_model_summary(file.path(model_dir, model_run))
  model_path <- file.path(model_dir, model_run)
  info <- get_model_parameter_info(model_path)
  table <- get_parameters(file.path(model_dir, model_run)) |>
    apply_table_spec(info, spec) |>
    add_summary_info(mod_sum) |>
    make_parameter_table()
  snapshot_gt_png(table, "parameter-table-ci-70")

  spec <- TableSpec(
    display_transforms = list(omega = c("cv")),
    sections = section_rules(
      kind == "THETA" ~ "Structural model parameters",
      kind == "OMEGA" & diagonal ~ "Interindividual variance parameters",
      kind == "OMEGA" & !diagonal ~ "Interindividual covariance parameters",
      kind == "SIGMA" ~ "Residual error",
      TRUE ~ "Other"
    ),
    n_sigfig = 3
  )

  mod1 <- read_model(file.path(model_dir, "run002"))
  mod_sum1 <- get_model_summary(file.path(model_dir, "run002"))
  info1 <- get_model_parameter_info(
    file.path(model_dir, "run002")
  )

  mod2 <- read_model(model_path)
  mod_sum2 <- get_model_summary(file.path(model_dir, model_run))
  info2 <- get_model_parameter_info(model_path)

  comp <- get_parameters(file.path(model_dir, "run002")) |>
    apply_table_spec(info1, spec) |>
    add_summary_info(mod_sum1) |>
    compare_with(
      get_parameters(file.path(model_dir, "run003")) |>
        apply_table_spec(info2, spec) |>
        add_summary_info(mod_sum2),
      labels = c(mod1$filename, mod2$filename)
    )

  table <- comp |>
    make_comparison_table()
  snapshot_gt_png(table, "parameter-table-comparison")

  spec <- TableSpec(
    display_transforms = list(omega = c("cv")),
    sections = section_rules(
      kind == "THETA" ~ "Structural model parameters",
      kind == "OMEGA" & diagonal ~ "Interindividual variance parameters",
      kind == "OMEGA" & !diagonal ~ "Interindividual covariance parameters",
      kind == "SIGMA" ~ "Residual variance",
      TRUE ~ "Other"
    ),
    ci_level = 0.7,
    n_sigfig = 3,
    name_source = "display"
  )

  mod_sum <- get_model_summary(file.path(model_dir, model_run))
  model_path <- file.path(model_dir, model_run)
  info <- get_model_parameter_info(model_path, lookup_path)

  table <- get_parameters(file.path(model_dir, model_run)) |>
    apply_table_spec(info, spec) |>
    add_summary_info(mod_sum, show_cond_num = FALSE) |>
    make_parameter_table()
  snapshot_gt_png(table, "parameter-table-summary-no-cond")

  table <- get_parameters(file.path(model_dir, model_run)) |>
    apply_table_spec(info, spec) |>
    add_summary_info(mod_sum, show_cond_num = FALSE, show_ofv = FALSE) |>
    make_parameter_table()
  snapshot_gt_png(table, "parameter-table-summary-no-cond-ofv")

  table <- get_parameters(file.path(model_dir, model_run)) |>
    apply_table_spec(info, spec) |>
    add_summary_info(mod_sum, show_method = FALSE) |>
    make_parameter_table()
  snapshot_gt_png(table, "parameter-table-summary-no-method")

  spec <- TableSpec(
    display_transforms = list(omega = c("cv")),
    sections = section_rules(
      kind == "THETA" ~ "Structural model parameters",
      kind == "OMEGA" & diagonal ~ "Interindividual variance parameters",
      kind == "OMEGA" & !diagonal ~ "Interindividual covariance parameters",
      kind == "SIGMA" ~ "Residual variance",
      TRUE ~ "Other"
    ),
    n_sigfig = 3
  )

  mod_sum <- get_model_summary(file.path(model_dir, model_run))
  model_path <- file.path(model_dir, model_run)
  info <- get_model_parameter_info(model_path)

  child_sum <- get_model_summary(file.path(model_dir, "run003b1"))
  child_info <- get_model_parameter_info(
    file.path(model_dir, "run003b1")
  )

  table <- get_parameters(file.path(model_dir, model_run)) |>
    apply_table_spec(info, spec) |>
    add_summary_info(mod_sum) |>
    compare_with(
      get_parameters(file.path(model_dir, "run003b1")) |>
        apply_table_spec(child_info, spec) |>
        add_summary_info(child_sum),
      labels = c("run003", "run003b1")
    ) |>
    make_comparison_table()
  snapshot_gt_png(table, "parameter-table-comparison-child")
})
