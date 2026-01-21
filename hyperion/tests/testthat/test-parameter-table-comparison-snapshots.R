test_that("parameter comparison table: run002 vs run003b1", {
  model_dir <- system.file("extdata",
    "models",
    "onecmt",
    package = "hyperion"
  )

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
		drop_columns = c("variability", "shrinkage")
  )

  mod1 <- read_model(file.path(model_dir, "run002.mod"))
  mod_sum1 <- get_model_summary(file.path(model_dir, "run002"))
  info1 <- get_model_parameter_info(file.path(model_dir, "run002"))

  mod2 <- read_model(file.path(model_dir, "run003b1.mod"))
  mod_sum2 <- get_model_summary(file.path(model_dir, "run003b1"))
  info2 <- get_model_parameter_info(file.path(model_dir, "run003b1"))

  comp <- get_parameters(file.path(model_dir, "run002")) |>
    apply_table_spec(spec, info1) |>
    add_summary_info(mod_sum1) |>
    compare_with(
      get_parameters(file.path(model_dir, "run003b1")) |>
        apply_table_spec(spec, info2) |>
        add_summary_info(mod_sum2),
      labels = c(mod1$filename, mod2$filename)
    )

  snapshot_gt(make_comparison_table(comp), "param-compare-grandparent")
})

test_that("parameter comparison table: run003 vs run003b1", {
  model_dir <- system.file("extdata",
    "models",
    "onecmt",
    package = "hyperion"
  )

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
		drop_columns = c("variability", "shrinkage")
  )

  mod_sum <- get_model_summary(file.path(model_dir, "run003"))
  info <- get_model_parameter_info(file.path(model_dir, "run003"))

  child_sum <- get_model_summary(file.path(model_dir, "run003b1"))
  child_info <- get_model_parameter_info(file.path(model_dir, "run003b1"))

  comp <- get_parameters(file.path(model_dir, "run003")) |>
    apply_table_spec(spec, info) |>
    add_summary_info(mod_sum) |>
    compare_with(
      get_parameters(file.path(model_dir, "run003b1")) |>
        apply_table_spec(spec, child_info) |>
        add_summary_info(child_sum),
      labels = c("run003", "run003b1")
    )

  snapshot_gt(make_comparison_table(comp), "param-compare-child")
})

test_that("parameter comparison table: run002 vs run003b1 drop symbol", {
  model_dir <- system.file("extdata",
    "models",
    "onecmt",
    package = "hyperion"
  )

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
		drop_columns = c("symbol", "variability", "shrinkage")

  )

  mod1 <- read_model(file.path(model_dir, "run002.mod"))
  mod_sum1 <- get_model_summary(file.path(model_dir, "run002"))
  info1 <- get_model_parameter_info(file.path(model_dir, "run002"))

  mod2 <- read_model(file.path(model_dir, "run003b1.mod"))
  mod_sum2 <- get_model_summary(file.path(model_dir, "run003b1"))
  info2 <- get_model_parameter_info(file.path(model_dir, "run003b1"))

  comp <- get_parameters(file.path(model_dir, "run002")) |>
    apply_table_spec(spec, info1) |>
    add_summary_info(mod_sum1) |>
    compare_with(
      get_parameters(file.path(model_dir, "run003b1")) |>
        apply_table_spec(spec, info2) |>
        add_summary_info(mod_sum2),
      labels = c(mod1$filename, mod2$filename)
    )

  snapshot_gt(make_comparison_table(comp), "param-compare-no-symbol")
})

test_that("parameter comparison table: run002 vs run003b1 drop configurable", {
  model_dir <- system.file("extdata",
    "models",
    "onecmt",
    package = "hyperion"
  )

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
    drop_columns = c("variability", "shrinkage", "pct_change", "symbol_left", "rse_right")
  )

  mod1 <- read_model(file.path(model_dir, "run002.mod"))
  mod_sum1 <- get_model_summary(file.path(model_dir, "run002"))
  info1 <- get_model_parameter_info(file.path(model_dir, "run002"))

  mod2 <- read_model(file.path(model_dir, "run003b1.mod"))
  mod_sum2 <- get_model_summary(file.path(model_dir, "run003b1"))
  info2 <- get_model_parameter_info(file.path(model_dir, "run003b1"))

  comp <- get_parameters(file.path(model_dir, "run002")) |>
    apply_table_spec(spec, info1) |>
    add_summary_info(mod_sum1) |>
    compare_with(
      get_parameters(file.path(model_dir, "run003b1")) |>
        apply_table_spec(spec, info2) |>
        add_summary_info(mod_sum2),
      labels = c(mod1$filename, mod2$filename)
    )

  snapshot_gt(make_comparison_table(comp), "param-compare-drop-cols")
})

test_that("parameter comparison table: three models with reference_model", {
  model_dir <- system.file("extdata",
    "models",
    "onecmt",
    package = "hyperion"
  )

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
    drop_columns = c("variability", "shrinkage")
  )

  # Get model summaries and parameter info for all three models
  sum1 <- get_model_summary(file.path(model_dir, "run001"))
  info1 <- get_model_parameter_info(file.path(model_dir, "run001"))

  sum2 <- get_model_summary(file.path(model_dir, "run002"))
  info2 <- get_model_parameter_info(file.path(model_dir, "run002"))

  sum3 <- get_model_summary(file.path(model_dir, "run003"))
  info3 <- get_model_parameter_info(file.path(model_dir, "run003"))

  # Build comparison: run001 + run002 (vs run001) + run003 (vs run001)
  comp <- get_parameters(file.path(model_dir, "run001")) |>
    apply_table_spec(spec, info1) |>
    add_summary_info(sum1) |>
    compare_with(
      get_parameters(file.path(model_dir, "run002")) |>
        apply_table_spec(spec, info2) |>
        add_summary_info(sum2),
      labels = c("run001", "run002")
    ) |>
    compare_with(
      get_parameters(file.path(model_dir, "run003")) |>
        apply_table_spec(spec, info3) |>
        add_summary_info(sum3),
      labels = "run003",
      reference_model = "run001"
    )

  snapshot_gt(make_comparison_table(comp), "param-compare-ref-mod")
})
