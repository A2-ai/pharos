test_that("hyperion.nonmem-summary knit_print works", {
  model_root <- testthat::test_path("testdata", "models", "onecmt")
  mods <- list.dirs(model_root, recursive = FALSE)
  mods <- mods[vapply(
    mods,
    function(p) {
      length(list.files(p, pattern = "\\.(mod|ctl)$", ignore.case = TRUE)) > 0
    },
    logical(1)
  )]

  for (p in mods) {
    mod_sum <- get_model_summary(p)
    model_name <- basename(p)
    snapshot_knit_html(mod_sum, paste0("summary-knit-", model_name))
  }
})
