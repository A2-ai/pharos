test_that("parameter table: base display", {
  model_dir <- system.file("extdata", "models", "onecmt", package = "hyperion")
  model_run <- "run003"
  lookup_path <- system.file("lookup.toml", package = "hyperion")

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

  info <- get_model_parameter_info(
    file.path(model_dir, model_run),
    lookup_path
  )
  info@sigma$`SIGMA(1,1)`@parameterization <- "Proportional"

  mod_sum <- get_model_summary(file.path(model_dir, model_run))

  table <- get_parameters(file.path(model_dir, model_run)) |>
    apply_table_spec(spec, info) |>
    add_summary_info(mod_sum) |>
    make_parameter_table()
  snapshot_gt(table, "param-base-gt")
})

test_that("parameter table: display name source", {
  model_dir <- system.file("extdata", "models", "onecmt", package = "hyperion")

  model_run <- "run003"
  lookup_path <- system.file("lookup.toml", package = "hyperion")

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

  info <- get_model_parameter_info(
    file.path(model_dir, model_run),
    lookup_path
  )
  info@sigma$`SIGMA(1,1)`@parameterization <- "Proportional"
  info@sigma$`SIGMA(1,1)`@display <- "Proportional Error"
  info@sigma$`SIGMA(2,2)`@display <- "Additive Error"

  mod_sum <- get_model_summary(file.path(model_dir, model_run))

  table <- get_parameters(file.path(model_dir, model_run)) |>
    apply_table_spec(spec, info) |>
    add_summary_info(mod_sum) |>
    make_parameter_table()
  snapshot_gt(table, "param-display-gt")
})

test_that("parameter table: nonmem name source", {
  model_dir <- system.file("extdata", "models", "onecmt", package = "hyperion")

  model_run <- "run003"
  lookup_path <- system.file("lookup.toml", package = "hyperion")

  spec <- TableSpec(
    display_transforms = list(omega = c("cv")),
    sections = section_rules(
      kind == "THETA" ~ "Structural model parameters",
      kind == "OMEGA" & diagonal ~ "Interindividual variance parameters",
      kind == "OMEGA" & !diagonal ~ "Interindividual covariance parameters",
      kind == "SIGMA" ~ "Residual error",
      TRUE ~ "Other"
    ),
    name_source = "nonmem_name",
    drop_columns = "rse",
    title = paste(model_run, "Parameters")
  )

  info <- get_model_parameter_info(
    file.path(model_dir, model_run),
    lookup_path
  )
  info@sigma$`SIGMA(1,1)`@parameterization <- "Proportional"

  mod_sum <- get_model_summary(file.path(model_dir, model_run))

  table <- get_parameters(file.path(model_dir, model_run)) |>
    apply_table_spec(spec, info) |>
    add_summary_info(mod_sum) |>
    make_parameter_table()
  snapshot_gt(table, "param-nonmem-name-gt")
})

test_that("parameter table: description column", {
  model_dir <- system.file("extdata", "models", "onecmt", package = "hyperion")
  model_run <- "run003"
  lookup_path <- system.file("lookup.toml", package = "hyperion")

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
    add_columns = "description",
    drop_columns = "rse",
    title = paste(model_run, "Parameters")
  )

  info <- get_model_parameter_info(
    file.path(model_dir, model_run),
    lookup_path
  )
  info@sigma$`SIGMA(1,1)`@parameterization <- "Proportional"

  mod_sum <- get_model_summary(file.path(model_dir, model_run))

  table <- get_parameters(file.path(model_dir, model_run)) |>
    apply_table_spec(spec, info) |>
    add_summary_info(mod_sum) |>
    make_parameter_table()
  snapshot_gt(table, "param-description-gt")
})

test_that("parameter table: drop unit column", {
  model_dir <- system.file("extdata", "models", "onecmt", package = "hyperion")

  model_run <- "run003"
  lookup_path <- system.file("lookup.toml", package = "hyperion")

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
    drop_columns = "unit",
    title = paste(model_run, "Parameters")
  )

  info <- get_model_parameter_info(
    file.path(model_dir, model_run),
    lookup_path
  )
  info@sigma$`SIGMA(1,1)`@parameterization <- "Proportional"

  mod_sum <- get_model_summary(file.path(model_dir, model_run))

  table <- get_parameters(file.path(model_dir, model_run)) |>
    apply_table_spec(spec, info) |>
    add_summary_info(mod_sum) |>
    make_parameter_table()
  snapshot_gt(table, "param-drop-unit-gt")
})

test_that("parameter table: drop unit and shrinkage columns", {
  model_dir <- system.file("extdata", "models", "onecmt", package = "hyperion")
  model_run <- "run003"
  lookup_path <- system.file("lookup.toml", package = "hyperion")

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
    drop_columns = c("unit", "shrinkage"),
    title = paste(model_run, "Parameters")
  )

  info <- get_model_parameter_info(
    file.path(model_dir, model_run),
    lookup_path
  )
  info@sigma$`SIGMA(1,1)`@parameterization <- "Proportional"

  mod_sum <- get_model_summary(file.path(model_dir, model_run))

  table <- get_parameters(file.path(model_dir, model_run)) |>
    apply_table_spec(spec, info) |>
    add_summary_info(mod_sum) |>
    make_parameter_table()
  snapshot_gt(table, "param-drop-unit-shrink-gt")
})

test_that("parameter table: structural-only filter", {
  model_dir <- system.file("extdata", "models", "onecmt", package = "hyperion")
  model_run <- "run003"
  lookup_path <- system.file("lookup.toml", package = "hyperion")

  info <- get_model_parameter_info(
    file.path(model_dir, model_run),
    lookup_path
  )
  info@sigma$`SIGMA(1,1)`@parameterization <- "Proportional"

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
    apply_table_spec(sp_spec, info) |>
    make_parameter_table()
  snapshot_gt(table, "param-structural-only-gt")
})

test_that("parameter table: random effects only", {
  model_dir <- system.file("extdata", "models", "onecmt", package = "hyperion")

  model_run <- "run003"
  lookup_path <- system.file("lookup.toml", package = "hyperion")

  info <- get_model_parameter_info(
    file.path(model_dir, model_run),
    lookup_path
  )
  info@sigma$`SIGMA(1,1)`@parameterization <- "Proportional"

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
    apply_table_spec(re_spec, info) |>
    make_parameter_table()
  snapshot_gt(table, "param-random-effects-gt")
})

test_that("parameter table: 70% CI", {
  model_dir <- system.file("extdata", "models", "onecmt", package = "hyperion")
  model_run <- "run003"

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
  info <- get_model_parameter_info(file.path(model_dir, model_run))

  table <- get_parameters(file.path(model_dir, model_run)) |>
    apply_table_spec(spec, info) |>
    add_summary_info(mod_sum) |>
    make_parameter_table()
  snapshot_gt(table, "param-ci-70-gt")
})

test_that("parameter table: summary info without condition number", {
  model_dir <- system.file("extdata", "models", "onecmt", package = "hyperion")
  model_run <- "run003"
  lookup_path <- system.file("lookup.toml", package = "hyperion")

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
  info <- get_model_parameter_info(
    file.path(model_dir, model_run),
    lookup_path
  )

  table <- get_parameters(file.path(model_dir, model_run)) |>
    apply_table_spec(spec, info) |>
    add_summary_info(mod_sum, show_cond_num = FALSE) |>
    make_parameter_table()
  snapshot_gt(table, "param-sum-no-cond-gt")
})

test_that("parameter table: summary info without condition number or OFV", {
  model_dir <- system.file("extdata", "models", "onecmt", package = "hyperion")
  model_run <- "run003"
  lookup_path <- system.file("lookup.toml", package = "hyperion")

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
  info <- get_model_parameter_info(
    file.path(model_dir, model_run),
    lookup_path
  )

  table <- get_parameters(file.path(model_dir, model_run)) |>
    apply_table_spec(spec, info) |>
    add_summary_info(mod_sum, show_cond_num = FALSE, show_ofv = FALSE) |>
    make_parameter_table()
  snapshot_gt(table, "param-sum-no-cond-ofv-gt")
})

test_that("parameter table: summary info without method", {
  model_dir <- system.file("extdata", "models", "onecmt", package = "hyperion")
  model_run <- "run003"
  lookup_path <- system.file("lookup.toml", package = "hyperion")

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
  info <- get_model_parameter_info(
    file.path(model_dir, model_run),
    lookup_path
  )

  table <- get_parameters(file.path(model_dir, model_run)) |>
    apply_table_spec(spec, info) |>
    add_summary_info(mod_sum, show_method = FALSE) |>
    make_parameter_table()
  snapshot_gt(table, "param-sum-no-method-gt")
})

test_that("symbol + fixed shows nicely", {
  model_dir <- system.file("extdata", "models", "onecmt", package = "hyperion")
  model_run <- "run001"
  lookup_path <- system.file("lookup.toml", package = "hyperion")

  lookup_path <- system.file("lookup.toml", package = "hyperion")

  spec <- TableSpec(
    columns = c("name", "symbol", "fixed"),
    sections = section_rules(
      kind == "THETA" ~ "Structural model parameters",
      kind == "OMEGA" & diagonal ~ "Interindividual variance parameters",
      kind == "OMEGA" & !diagonal ~ "Interindividual covariance parameters",
      kind == "SIGMA" ~ "Residual error",
      TRUE ~ "Other"
    )
  )

  mod_sum <- get_model_summary(file.path(model_dir, model_run))
  info <- get_model_parameter_info(
    file.path(model_dir, model_run),
    lookup_path
  )

  table <- get_parameters(file.path(model_dir, model_run)) |>
    apply_table_spec(spec, info) |>
    add_summary_info(mod_sum) |>
    make_parameter_table()

  snapshot_gt(table, "param-symbol-fixed-gt")
})
