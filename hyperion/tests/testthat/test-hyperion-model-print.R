test_that("hyperion.nonmem-model print works", {
  mod_dir <- system.file("extdata", "mod", package = "hyperion")
  mods <- list.files(mod_dir, full.names = TRUE)

  for (p in mods) {
    mod <- read_model(p)
		model_name <- tools::file_path_sans_ext(basename(p))
    expect_snapshot(mod, variant = model_name)
  }
})
