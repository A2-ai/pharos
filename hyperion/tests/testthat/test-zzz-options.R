test_that(".onLoad sets default options when missing", {
  withr::local_options(list(
    hyperion.significant_number_display = NULL,
    hyperion.nonmem_model.show_included_columns = NULL,
    hyperion.nonmem_summary.rse_threshold = NULL,
    hyperion.nonmem_summary.shrinkage_threshold = NULL
  ))

  hyperion:::.onLoad("", "")

  expect_equal(getOption("hyperion.significant_number_display"), 4L)
  expect_equal(getOption("hyperion.nonmem_model.show_included_columns"), FALSE)
  expect_equal(getOption("hyperion.nonmem_summary.rse_threshold"), 50)
  expect_equal(getOption("hyperion.nonmem_summary.shrinkage_threshold"), 30)
})
