test_that("parameter comparison table: run002 vs run003b1", {
  model_dir <- file.path("testdata", "models", "onecmt")

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

  mod1 <- read_model(file.path(model_dir, "run002"))
  mod_sum1 <- get_model_summary(file.path(model_dir, "run002"))
  info1 <- get_model_parameter_info(file.path(model_dir, "run002"))

  mod2 <- read_model(file.path(model_dir, "run003b1"))
  mod_sum2 <- get_model_summary(file.path(model_dir, "run003b1"))
  info2 <- get_model_parameter_info(file.path(model_dir, "run003b1"))

  comp <- get_parameters(file.path(model_dir, "run002")) |>
    apply_table_spec(info1, spec) |>
    add_summary_info(mod_sum1) |>
    compare_with(
      get_parameters(file.path(model_dir, "run003b1")) |>
        apply_table_spec(info2, spec) |>
        add_summary_info(mod_sum2),
      labels = c(mod1$filename, mod2$filename)
    )

  snapshot_gt_png(make_comparison_table(comp), "param-compare-grandparent")
})

test_that("parameter comparison table: run003 vs run003b1", {
  model_dir <- file.path("testdata", "models", "onecmt")

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

  mod_sum <- get_model_summary(file.path(model_dir, "run003"))
  info <- get_model_parameter_info(file.path(model_dir, "run003"))

  child_sum <- get_model_summary(file.path(model_dir, "run003b1"))
  child_info <- get_model_parameter_info(file.path(model_dir, "run003b1"))

  comp <- get_parameters(file.path(model_dir, "run003")) |>
    apply_table_spec(info, spec) |>
    add_summary_info(mod_sum) |>
    compare_with(
      get_parameters(file.path(model_dir, "run003b1")) |>
        apply_table_spec(child_info, spec) |>
        add_summary_info(child_sum),
      labels = c("run003", "run003b1")
    )

  snapshot_gt_png(make_comparison_table(comp), "param-compare-child")
})

test_that("parameter comparison table: run002 vs run003b1 drop symbol", {
  model_dir <- file.path("testdata", "models", "onecmt")

  spec <- TableSpec(
    display_transforms = list(omega = c("cv")),
    sections = section_rules(
      kind == "THETA" ~ "Structural model parameters",
      kind == "OMEGA" & diagonal ~ "Interindividual variance parameters",
      kind == "OMEGA" & !diagonal ~ "Interindividual covariance parameters",
      kind == "SIGMA" ~ "Residual variance",
      TRUE ~ "Other"
    ),
    n_sigfig = 3,
    drop_columns = "symbol"
  )

  mod1 <- read_model(file.path(model_dir, "run002"))
  mod_sum1 <- get_model_summary(file.path(model_dir, "run002"))
  info1 <- get_model_parameter_info(file.path(model_dir, "run002"))

  mod2 <- read_model(file.path(model_dir, "run003b1"))
  mod_sum2 <- get_model_summary(file.path(model_dir, "run003b1"))
  info2 <- get_model_parameter_info(file.path(model_dir, "run003b1"))

  comp <- get_parameters(file.path(model_dir, "run002")) |>
    apply_table_spec(info1, spec) |>
    add_summary_info(mod_sum1) |>
    compare_with(
      get_parameters(file.path(model_dir, "run003b1")) |>
        apply_table_spec(info2, spec) |>
        add_summary_info(mod_sum2),
      labels = c(mod1$filename, mod2$filename)
    )

  snapshot_gt_png(make_comparison_table(comp), "param-compare-no-symbol")
})

test_that("parameter comparison table: run002 vs run003b1 drop configurable", {
  model_dir <- file.path("testdata", "models", "onecmt")

  spec <- TableSpec(
    display_transforms = list(omega = c("cv")),
    sections = section_rules(
      kind == "THETA" ~ "Structural model parameters",
      kind == "OMEGA" & diagonal ~ "Interindividual variance parameters",
      kind == "OMEGA" & !diagonal ~ "Interindividual covariance parameters",
      kind == "SIGMA" ~ "Residual variance",
      TRUE ~ "Other"
    ),
    n_sigfig = 3,
    drop_columns = c("pct_change", "symbol_left", "rse_right")
  )

  mod1 <- read_model(file.path(model_dir, "run002"))
  mod_sum1 <- get_model_summary(file.path(model_dir, "run002"))
  info1 <- get_model_parameter_info(file.path(model_dir, "run002"))

  mod2 <- read_model(file.path(model_dir, "run003b1"))
  mod_sum2 <- get_model_summary(file.path(model_dir, "run003b1"))
  info2 <- get_model_parameter_info(file.path(model_dir, "run003b1"))

  comp <- get_parameters(file.path(model_dir, "run002")) |>
    apply_table_spec(info1, spec) |>
    add_summary_info(mod_sum1) |>
    compare_with(
      get_parameters(file.path(model_dir, "run003b1")) |>
        apply_table_spec(info2, spec) |>
        add_summary_info(mod_sum2),
      labels = c(mod1$filename, mod2$filename)
    )

  snapshot_gt_png(make_comparison_table(comp), "param-compare-drop-cols")
})
