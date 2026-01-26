test_that("hyperion.nonmem-summary knit_print works", {
  model_root <- testthat::test_path("testdata", "models", "onecmt")
  mods <- list.files(
    model_root,
    pattern = "\\.(mod|ctl)$",
    ignore.case = TRUE,
    full.names = TRUE
  )

  mods <- lapply(
    mods,
    function(p) {
      read_model(p)
    }
  )

  for (p in mods) {
    mod_sum <- summary(p)
    model_name <- attr(p, "filename")
    snapshot_knit_html(mod_sum, paste0("summary-knit-", model_name))
  }
})
