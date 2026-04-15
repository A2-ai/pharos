# bootstrap.R - Run before package build (Config/build/bootstrap: TRUE)
# Respect caller's NOT_CRAN if explicitly set; default to true (dev mode).
# bootstrap.R only runs during R CMD build (Config/build/bootstrap: TRUE),
# never on CRAN install, so defaulting to true is safe.
# PREPARE_CRAN=false prevents accidental inheritance of release-prep mode.

not_cran <- Sys.getenv("NOT_CRAN", unset = "")
if (!nzchar(not_cran)) not_cran <- "true"

env <- c(NOT_CRAN = not_cran, PREPARE_CRAN = "false")
env_strings <- paste0(names(env), "=", env)

if (.Platform$OS.type == "windows") {
  system2("bash", c("-l", "-c", "./configure.win"), env = env_strings)
} else {
  system2("./configure", env = env_strings)
}
