.onLoad <- function(libname, pkgname) {
  set_panic_message()

  # Set default hyperion options if not already set
  if (is.null(getOption("hyperion.significant_number_display"))) {
    options(hyperion.significant_number_display = 4L)
  }

  # Set default hyperion nonmem options if not already set
  if (is.null(getOption("hyperion.nonmem_model.show_included_columns"))) {
    options(hyperion.nonmem_model.show_included_columns = FALSE)
  }

  if (is.null(getOption("hyperion.nonmem_summary.rse_threshold"))) {
    options(hyperion.nonmem_summary.rse_threshold = 50)
  }

  if (is.null(getOption("hyperion.nonmem_summary.shrinkage_threshold"))) {
    options(hyperion.nonmem_summary.shrinkage_threshold = 30)
  }
}

.onAttach <- function(libname, pkgname) {
  msg <- hyperion_options_message()
  packageStartupMessage(msg)
}
