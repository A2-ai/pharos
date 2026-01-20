test_that("hyperion_nonmem_audit print works", {
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
    info <- get_model_parameter_info(p)
    audit <- audit_parameter_info(info)
    expect_snapshot(print(audit))
  }
})
