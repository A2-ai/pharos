#' Apply lookup defaults to a ModelComments object
#'
#' Convenience function to apply lookup defaults to all comments in a
#' ModelComments object (theta, omega, sigma).
#'
#' @param info A ModelComments object
#' @param lookup_path Path to a toml lookup file
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
#' lookup toml file. Matches the comment's `name` field against toml keys.
#'
#' @param comment A ThetaComment, OmegaComment, or SigmaComment object
#' @param lookup_path Path to a toml lookup file
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

  lookup <- load_lookup_toml(lookup_path)
  lookup_path <- relative_path(lookup_path)

  # Try to find entry by user name first, then by NONMEM name
  entry <- NULL

  lookup_names <- names(lookup)
  lookup_lower <- tolower(lookup_names)

  if (!is.null(comment@name)) {
    match_idx <- match(tolower(comment@name), lookup_lower)
    if (!is.na(match_idx)) {
      entry <- lookup[[lookup_names[match_idx]]]
    }
  }
  if (
    is.null(entry) &&
      !is.null(comment@nonmem_name)
  ) {
    match_idx <- match(tolower(comment@nonmem_name), lookup_lower)
    if (!is.na(match_idx)) {
      entry <- lookup[[lookup_names[match_idx]]]
    }
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
      kind <- if (is_theta) "THETA" else if (is_omega) "OMEGA" else "SIGMA"
      mapped <- map_parameterization(entry$parameterization, kind)
      if (!is.null(mapped)) {
        comment@parameterization <- mapped
        attr(comment, "sources")$parameterization <- lookup_path
      }
    }
  }

  comment
}

#' @noRd
load_lookup_toml <- function(path) {
  if (!requireNamespace("tomledit", quietly = TRUE)) {
    stop("Package 'tomledit' is required for using a lookup toml file")
  }

  if (!file.exists(path)) {
    stop("Lookup file not found: ", path)
  }
  tomledit::from_toml(tomledit::read_toml(path))
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

#' Add a parameter definition to a lookup file
#'
#' Adds or updates a parameter entry in a TOML lookup file. The parameterization
#' value is validated against known types.
#'
#' @param path Path to the lookup.toml file
#' @param name Parameter name (e.g., "TVCL", "OM1")
#' @param display Display name for tables (optional)
#' @param desc Description text (optional)
#' @param unit Unit string (optional)
#' @param parameterization Parameterization type (optional). Accepts raw values
#'   (EXP, LOG, PROP, ADD, etc.) or canonical values (LogNormal, Proportional,
#'   AddErr, LogAddErr, Logit, Identity).
#' @param overwrite If TRUE, overwrite existing entry. Default FALSE.
#' @return Invisibly returns the path to the lookup file
#' @export
add_parameter_to_lookup <- function(
  path,
  name,

  display = NULL,
  desc = NULL,
  unit = NULL,
  parameterization = NULL,
  overwrite = FALSE
) {
  if (!requireNamespace("tomledit", quietly = TRUE)) {
    stop("Package 'tomledit' is required for add_parameter_to_lookup()")
  }

  if (missing(name) || !nzchar(name)) {
    stop("name is required")
  }

  # Validate parameterization if provided
  if (!is.null(parameterization)) {
    mapped <- map_parameterization(parameterization, "THETA")
    if (is.null(mapped)) {
      stop(
        "Invalid parameterization: ",
        parameterization,
        ". Valid values: ",
        paste(valid_parameterizations(), collapse = ", ")
      )
    }
    parameterization <- mapped
  }

  # Read existing file or create new
  if (file.exists(path)) {
    toml <- tomledit::read_toml(path)
    existing <- tomledit::from_toml(toml)

    if (name %in% names(existing) && !overwrite) {
      stop(
        "Parameter '",
        name,
        "' already exists in lookup. Use overwrite = TRUE to replace."
      )
    }
  } else {
    toml <- tomledit::toml()
  }

  # Build entry list (only include non-NULL fields)
  entry <- list()
  if (!is.null(display)) entry$display <- display
  if (!is.null(desc)) entry$desc <- desc
  if (!is.null(unit)) entry$unit <- unit
  if (!is.null(parameterization)) entry$parameterization <- parameterization

  if (length(entry) == 0) {
    stop(
      "At least one of display, desc, unit, or parameterization must be provided"
    )
  }

  # Insert the entry using named list
  args <- list(toml)
  args[[name]] <- entry
  toml <- do.call(tomledit::insert_items, args)

  # Write back
  tomledit::write_toml(toml, path)

  invisible(path)
}

#' Remove a parameter definition from a lookup file
#'
#' Removes a parameter entry from a TOML lookup file.
#'
#' @param path Path to the lookup.toml file
#' @param name Parameter name to remove
#' @return Invisibly returns the path to the lookup file
#' @export
remove_parameter_from_lookup <- function(path, name) {
  if (!requireNamespace("tomledit", quietly = TRUE)) {
    stop("Package 'tomledit' is required for remove_parameter_from_lookup()")
  }

  if (!file.exists(path)) {
    stop("Lookup file not found: ", path)
  }

  toml <- tomledit::read_toml(path)
  existing <- tomledit::from_toml(toml)

  if (!name %in% names(existing)) {
    warning("Parameter '", name, "' not found in lookup file")
    return(invisible(path))
  }

  toml <- tomledit::remove_items(toml, name)
  tomledit::write_toml(toml, path)

  invisible(path)
}

#' List all parameters in a lookup file
#'
#' Returns the names of all parameter entries in a TOML lookup file.
#'
#' @param path Path to the lookup.toml file
#' @return Character vector of parameter names
#' @export
list_lookup_parameters <- function(path) {
  lookup <- load_lookup_toml(path)
  names(lookup)
}
