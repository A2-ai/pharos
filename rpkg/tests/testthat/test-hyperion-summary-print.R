test_that("hyperion.nonmem-summary print works", {
  model_root <- system.file("extdata", "models", "onecmt", package = "hyperion")
  run_dirs <- list.dirs(model_root, recursive = FALSE)

  # Get mod files that have corresponding run directories
  mods <- vapply(run_dirs, function(dir) {
    mod_file <- file.path(model_root, paste0(basename(dir), ".mod"))
    if (file.exists(mod_file)) mod_file else NA_character_
  }, character(1))
  mods <- mods[!is.na(mods)]

  mods <- lapply(mods, read_model)

  for (p in mods) {
    mod_sum <- summary(p)
    expect_snapshot(print(mod_sum))
  }
})

test_that("hyperion.nonmem-summary print works for run005 (not_run)", {
  mod_path <- system.file("extdata", "models", "onecmt", "run005.mod", package = "hyperion")
  mod <- read_model(mod_path)
  mod_sum <- summary(mod)
  expect_snapshot(print(mod_sum))
})

test_that("hyperion.nonmem-summary print works for run004 (running)", {
  mod_path <- system.file("extdata", "models", "onecmt", "run004.mod", package = "hyperion")
  mod <- read_model(mod_path)
  mod_sum <- summary(mod)
  expect_snapshot(print(mod_sum))
})

test_that("hyperion.nonmem-summary print fails gracefully for ill-formatted comments", {
  # Temporarily remove type1 so raw comment parsing is exercised
  config_path <- testthat::test_path("..", "pharos.toml")
  original <- readLines(config_path)
  withr::defer(writeLines(original, config_path))
  writeLines(gsub('type = "type1"', '', original), config_path)

  mod_path <- system.file("extdata", "models", "run-error-names", "run-err.mod", package = "hyperion")
  mod <- read_model(mod_path)

  expect_warning(
    mod_sum <- summary(mod),
    "Could not apply parameter names from model comments."
  )
  expect_snapshot(print(mod_sum))
})
