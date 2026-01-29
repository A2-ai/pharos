test_that("hyperion.nonmem-model knit_print works", {
  mod_dir <- system.file("extdata", "mod", package = "hyperion")
  mods <- list.files(mod_dir, pattern = "\\.mod$", full.names = TRUE)

  for (p in mods) {
    mod <- read_model(p)
    model_name <- tools::file_path_sans_ext(basename(p))
    snapshot_knit_html(mod, paste0("model-knit-", model_name))
  }
})
