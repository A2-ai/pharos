test_that("hyperion.nonmem-summary print works", {
  mods <- list.dirs("testdata/models", recursive = FALSE)
  mods <- mods[vapply(
    mods,
    function(p) {
      length(list.files(
        p,
        pattern = "\\.(mod|ctl)$",
        ignore.case = TRUE
      )) > 0
    },
    logical(1)
  )]

  for (p in mods) {
    mod_sum <- get_model_summary(p)
		model_name <- tools::file_path_sans_ext(basename(p))
    
		expect_snapshot(mod_sum, variant = model_name)
  }
})
