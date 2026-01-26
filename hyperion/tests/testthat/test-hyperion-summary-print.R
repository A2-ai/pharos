test_that("hyperion.nonmem-summary print works", {
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
    expect_snapshot(print(mod_sum))
  }
})
