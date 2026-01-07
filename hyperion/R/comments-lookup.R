#' Apply lookup defaults to a ModelComments object
#'
#' Convenience function to apply lookup defaults to all comments in a
#' ModelComments object (theta, omega, sigma).
#'
#' @param info A ModelComments object
#' @param lookup_path Path to a yaml lookup file
#' @return The modified ModelComments object
#' @export
apply_lookup <- function(info, lookup_path) {
  if (!S7::S7_inherits(info, ModelComments)) {
    stop("info must be a ModelComments object")
  }

  for (slot in c("theta", "omega", "sigma")) {
    comments <- S7::prop(info, slot)
    for (name in names(comments)) {
      comments[[name]] <- apply_lookup_defaults(comments[[name]], lookup_path)
    }
    S7::prop(info, slot) <- comments
  }

  info
}

#' Apply lookup defaults to a parameter comment
#'
#' Fills NULL fields (display, description, unit, parameterization) from a
#' lookup yaml file. Matches the comment's `name` field against yaml keys.
#'
#' @param comment A ThetaComment, OmegaComment, or SigmaComment object
#' @param lookup_path Path to a yaml lookup file
#' @return The modified comment object
#' @export
apply_lookup_defaults <- function(comment, lookup_path) {
  is_theta <- S7::S7_inherits(comment, ThetaComment)
  is_omega <- S7::S7_inherits(comment, OmegaComment)
  is_sigma <- S7::S7_inherits(comment, SigmaComment)

  if (!is_theta && !is_omega && !is_sigma) {
    stop(
      "comment must be a ThetaComment, OmegaComment, or SigmaComment object"
    )
  }

  lookup <- load_lookup_yaml(lookup_path)
  lookup_path <- relative_path(lookup_path)

  # Try to find entry by user name first, then by NONMEM name
  entry <- NULL

  if (!is.null(comment@name) && comment@name %in% names(lookup)) {
    entry <- lookup[[comment@name]]
  } else if (
    !is.null(comment@nonmem_name) && comment@nonmem_name %in% names(lookup)
  ) {
    entry <- lookup[[comment@nonmem_name]]
  }

  if (is.null(entry)) {
    return(comment)
  }

  # Initialize sources if missing (for comments created outside get_model_parameter_info)
  if (is.null(attr(comment, "sources"))) {
    attr(comment, "sources") <- list()
  }

  if (is.null(comment@display) && !is.null(entry$display)) {
    comment@display <- entry$display
    attr(comment, "sources")$display <- lookup_path
  }

  if (is.null(comment@description) && !is.null(entry$desc)) {
    comment@description <- entry$desc
    attr(comment, "sources")$description <- lookup_path
  }

  # Only theta has unit property
  if (is_theta && is.null(comment@unit) && !is.null(entry$unit)) {
    resolved_unit <- resolve_unit(entry$unit, lookup)
    if (!is.null(resolved_unit) && resolved_unit != "none") {
      comment@unit <- resolved_unit
      attr(comment, "sources")$unit <- lookup_path
    }
  }

  if (is.null(comment@parameterization) && !is.null(entry$parameterization)) {
    if (entry$parameterization != "none") {
      comment@parameterization <- entry$parameterization
      attr(comment, "sources")$parameterization <- lookup_path
    }
  }

  comment
}

#' @noRd
load_lookup_yaml <- function(path) {
  if (!requireNamespace("yaml", quietly = TRUE)) {
    stop("Package 'yaml' is required for using a lookup yaml file")
  }

  if (!file.exists(path)) {
    stop("Lookup file not found: ", path)
  }
  yaml::read_yaml(path)
}

#' @noRd
resolve_unit <- function(unit, lookup) {
  if (is.null(unit) || unit == "none") {
    return(NULL)
  }

  # Check if unit contains a reference (e.g., "VOLUME/TIME")
  if (grepl("/", unit)) {
    parts <- strsplit(unit, "/")[[1]]
    resolved_parts <- vapply(
      parts,
      function(p) {
        p <- trimws(p)
        if (p %in% names(lookup) && !is.null(lookup[[p]]$unit)) {
          resolve_unit(lookup[[p]]$unit, lookup)
        } else {
          p
        }
      },
      character(1)
    )
    return(paste(resolved_parts, collapse = "/"))
  }

  # Check if it's a direct reference
  if (unit %in% names(lookup) && !is.null(lookup[[unit]]$unit)) {
    return(resolve_unit(lookup[[unit]]$unit, lookup))
  }

  unit
}
