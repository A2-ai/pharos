snapshot_gt <- function(table, name) {
  testthat::skip_if_not_installed("gt")
  testthat::skip_if_not_installed("webshot2")

  path <- file.path(tempdir(), paste0(name, ".png"))
  gt::gtsave(table, filename = path, vwidth = 4000)

  testthat::expect_snapshot_file(path)
}

snapshot_flextable <- function(table, name) {
  testthat::skip_if_not_installed("flextable")
  testthat::skip_if_not_installed("webshot2")

  path <- file.path(tempdir(), paste0(name, ".png"))

  # Use webshot2 directly instead of save_as_image to properly render
 # KaTeX equations (which require browser rendering with CSS loading delay)
  html_path <- file.path(tempdir(), paste0(name, ".html"))
  flextable::save_as_html(table, path = html_path)

  webshot2::webshot(
    url = html_path,
    file = path,
    delay = 2,  # Allow time for KaTeX CSS to load from CDN
    vwidth = 1000
  )

  testthat::expect_snapshot_file(path)
}
