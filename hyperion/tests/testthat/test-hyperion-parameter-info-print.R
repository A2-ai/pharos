test_that("hyperion_nonmem_parameter_info print works", {
  model_root <- system.file("extdata", "models", "onecmt", package = "hyperion")
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
    expect_snapshot(print(info))
  }
})
