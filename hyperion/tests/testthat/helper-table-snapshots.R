snapshot_gt <- function(table, name) {
  testthat::skip_if_not_installed("gt")
  testthat::skip_if_not_installed("webshot2")

  path <- file.path(tempdir(), paste0(name, ".png"))
  html_path <- file.path(tempdir(), paste0(name, ".html"))
  gt::gtsave(table, filename = html_path)

  webshot2::webshot(
    url = html_path,
    file = path,
    selector = "table.gt_table",
    delay = 1,
		vwidth = 4000
  )

  testthat::expect_snapshot_file(path)
}

snapshot_flextable <- function(table, name) {
  testthat::skip_if_not_installed("flextable")
  testthat::skip_if_not_installed("htmltools")
  testthat::skip_if_not_installed("webshot2")

  path <- file.path(tempdir(), paste0(name, ".png"))

  # Use webshot2 directly instead of save_as_image to properly render
 # KaTeX equations (which require browser rendering with CSS loading delay)
  html_path <- file.path(tempdir(), paste0(name, ".html"))
  html_tag <- flextable::htmltools_value(table)
  htmltools::save_html(html_tag, file = html_path)

  webshot2::webshot(
    url = html_path,
    file = path,
    selector = ".flextable-shadow-host",
    delay = 1, # Allow time for KaTeX CSS to load from CDN
		vwidth = 2000
  )

  testthat::expect_snapshot_file(path)
}
