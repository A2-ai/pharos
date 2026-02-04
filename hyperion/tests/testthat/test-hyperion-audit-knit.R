test_that("hyperion_nonmem_audit knit_print works", {
  model_root <- system.file("extdata", "models", "onecmt", package = "hyperion")
  mods <- list.dirs(model_root, recursive = FALSE)
  # Only include completed runs (have pharos_end.json)
  mods <- mods[vapply(
    mods,
    function(p) {
      has_mod <- length(list.files(p, pattern = "\\.(mod|ctl)$", ignore.case = TRUE)) > 0
      has_end <- file.exists(file.path(p, "pharos_end.json"))
      has_mod && has_end
    },
    logical(1)
  )]

  for (p in mods) {
    info <- get_model_parameter_info(p)
    audit <- audit_parameter_info(info)
    model_name <- basename(p)
    snapshot_knit_html(audit, paste0("audit-knit-", model_name))
  }
})
