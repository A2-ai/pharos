test_that("hyperion.nonmem-model print works", {
  mod_dir <- system.file("extdata", "mod", package = "hyperion")
  mods <- list.files(mod_dir, pattern = "\\.mod$", full.names = TRUE)

  for (p in mods) {
    mod <- read_model(p)
    expect_snapshot(print(mod))
  }
})
