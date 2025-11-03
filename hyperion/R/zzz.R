.onLoad <- function(libname, pkgname) {
  set_panic_message()

  # Set default hyperion options if not already set
  if (is.null(getOption("hyperion.significant_number_display"))) {
    options(hyperion.significant_number_display = 4L)
  }
}

.onAttach <- function(libname, pkgname) {
  msg <- hyperion_options_message()
  packageStartupMessage(msg)
}
