#' Get all theta names from ModelComments
#'
#' @param model_comments A ModelComments object
#' @return Character vector of theta names (excluding NULL names)
#' @export
get_theta_names <- function(model_comments) {
  if (!S7::S7_inherits(model_comments, ModelComments)) {
    stop("model_comments must be a ModelComments object")
  }
  theta_names <- vapply(
    model_comments@theta,
    function(c) if (is.null(c@name)) NA_character_ else c@name,
    character(1)
  )
  theta_names[!is.na(theta_names)]
}

#' Get a comment by original NONMEM name
#'
#' @param model_comments A ModelComments object
#' @param nonmem_name The NONMEM parameter name (e.g., "THETA1", "OMEGA(1,1)")
#' @return The comment object (ThetaComment, OmegaComment, or SigmaComment), or NULL if not found
#' @export
get_comment <- function(model_comments, nonmem_name) {
  if (!S7::S7_inherits(model_comments, ModelComments)) {
    stop("model_comments must be a ModelComments object")
  }

  if (grepl("^THETA", nonmem_name)) {
    return(model_comments@theta[[nonmem_name]])
  } else if (grepl("^OMEGA", nonmem_name)) {
    return(model_comments@omega[[nonmem_name]])
  } else if (grepl("^SIGMA", nonmem_name)) {
    return(model_comments@sigma[[nonmem_name]])
  }

  NULL
}

#' @noRd
build_comment_lookup <- function(model_comments) {
  all_comments <- c(
    model_comments@theta,
    model_comments@omega,
    model_comments@sigma
  )

  by_user_name <- list()
  for (comment in all_comments) {
    if (!is.null(comment@name)) {
      by_user_name[[comment@name]] <- comment
    }
  }

  list(by_nonmem_name = all_comments, by_user_name = by_user_name)
}

#' Get parameterization (transform) for parameters by name
#'
#' @param model_comments A ModelComments object
#' @param names Character vector of parameter names. Can be user-defined names
#'   (e.g., "CL", "V", "OM1") or NONMEM names (e.g., "THETA1", "OMEGA(1,1)"),
#'   or a mix of both.
#' @return Character vector of parameterization values (e.g., "LogNormal",
#'   "Identity", "Proportional"). Returns NA for names not found.
#' @export
get_parameter_transform <- function(model_comments, names) {
  if (!S7::S7_inherits(model_comments, ModelComments)) {
    stop("model_comments must be a ModelComments object")
  }

  lookup <- build_comment_lookup(model_comments)
  by_nonmem_name <- lookup$by_nonmem_name
  by_user_name <- lookup$by_user_name

  # Look up each requested name
  vapply(
    names,
    function(nm) {
      # Handle format like "OM1 (TVCL)" - extract NONMEM name before space
      lookup_nm <- sub(" \\(.*\\)$", "", nm)

      # Try nonmem_name first
      comment <- by_nonmem_name[[lookup_nm]]
      if (is.null(comment)) {
        # Try user name
        comment <- by_user_name[[lookup_nm]]
      }
      if (is.null(comment)) {
        return(NA_character_)
      }
      # Default NULL to Identity at usage time
      comment@parameterization %||% "Identity"
    },
    character(1),
    USE.NAMES = FALSE
  )
}

#' Get unit for parameters by name
#'
#' @param model_comments A ModelComments object
#' @param names Character vector of parameter names. Can be user-defined names
#'   (e.g., "CL", "V", "OM1") or NONMEM names (e.g., "THETA1", "OMEGA(1,1)"),
#'   or a mix of both.
#' @return Character vector of unit values (e.g., "L/h", "L"). Returns NA for
#'   names not found.
#' @export
get_parameter_unit <- function(model_comments, names) {
  if (!S7::S7_inherits(model_comments, ModelComments)) {
    stop("model_comments must be a ModelComments object")
  }

  lookup <- build_comment_lookup(model_comments)
  by_nonmem_name <- lookup$by_nonmem_name
  by_user_name <- lookup$by_user_name

  # Look up each requested name
  vapply(
    names,
    function(nm) {
      # Handle format like "OM1 (TVCL)" - extract NONMEM name before space
      lookup_nm <- sub(" \\(.*\\)$", "", nm)

      # Try nonmem_name first
      comment <- by_nonmem_name[[lookup_nm]]
      if (is.null(comment)) {
        # Try user name
        comment <- by_user_name[[lookup_nm]]
      }
      if (is.null(comment)) {
        return(NA_character_)
      }
      # Only theta comments have unit property
      if (S7::S7_inherits(comment, ThetaComment)) {
        return(comment@unit %||% NA_character_)
      }
      NA_character_
    },
    character(1),
    USE.NAMES = FALSE
  )
}

#' Get parameter names from ModelComments
#'
#' Returns a data frame mapping NONMEM parameter names to user-defined
#' names and display names. Row names are set to NONMEM names for easy
#' access (e.g., `df["THETA1", "name"]`).
#'
#' @param model_comments A ModelComments object
#' @return Data frame with columns: name, display. Row names are NONMEM
#'   parameter names (e.g., "THETA1", "OMEGA(1,1)").
#' @export
get_parameter_names <- function(model_comments) {
  if (!S7::S7_inherits(model_comments, ModelComments)) {
    stop("model_comments must be a ModelComments object")
  }

  extract_row <- function(comment) {
    data.frame(
      name = comment@name %||% NA_character_,
      display = comment@display %||% NA_character_,
      stringsAsFactors = FALSE
    )
  }

  rows <- list()
  row_names <- character(0)

  for (nm in names(model_comments@theta)) {
    rows <- c(rows, list(extract_row(model_comments@theta[[nm]])))
    row_names <- c(row_names, nm)
  }

  for (nm in names(model_comments@omega)) {
    rows <- c(rows, list(extract_row(model_comments@omega[[nm]])))
    row_names <- c(row_names, nm)
  }

  for (nm in names(model_comments@sigma)) {
    rows <- c(rows, list(extract_row(model_comments@sigma[[nm]])))
    row_names <- c(row_names, nm)
  }

  if (length(rows) == 0) {
    return(data.frame(
      name = character(0),
      display = character(0),
      stringsAsFactors = FALSE
    ))
  }

  result <- do.call(rbind, rows)
  rownames(result) <- row_names
  result
}

#' Get ETA labels from model comments
#'
#' Creates ETA labels in the format "ETA1//ETA-CL" from diagonal omega parameters.
#' The label uses the associated_theta name if available, otherwise the omega name.
#'
#' @param model_comments A ModelComments object
#' @return Character vector of ETA labels (e.g., c("ETA1//ETA-CL", "ETA2//ETA-V"))
#' @export
get_eta_labels <- function(model_comments) {
  if (!S7::S7_inherits(model_comments, ModelComments)) {
    stop("model_comments must be a ModelComments object")
  }

  # Get diagonal omega elements only (where row == col)
  omega_names <- names(model_comments@omega)
  diagonal_omegas <- omega_names[vapply(
    omega_names,
    is_diagonal_omega,
    logical(1)
  )]

  # Sort numerically by row index
  diagonal_omegas <- diagonal_omegas[order(vapply(
    diagonal_omegas,
    function(nm) {
      match <- regmatches(nm, regexec("OMEGA\\((\\d+),(\\d+)\\)", nm))[[1]]
      if (length(match) >= 2) as.integer(match[2]) else 0L
    },
    integer(1)
  ))]

  # Build labels
  vapply(
    seq_along(diagonal_omegas),
    function(i) {
      omega_name <- diagonal_omegas[i]
      comment <- model_comments@omega[[omega_name]]

      # Use associated_theta if available, otherwise omega name
      suffix <- if (!is.null(comment@associated_theta)) {
        comment@associated_theta[1]
      } else if (!is.null(comment@name)) {
        comment@name
      } else {
        i
      }

      paste0("ETA", i, "//ETA-", suffix)
    },
    character(1)
  )
}
