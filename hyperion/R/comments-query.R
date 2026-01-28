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
  comments_by_kind <- list(
    THETA = model_comments@theta,
    OMEGA = model_comments@omega,
    SIGMA = model_comments@sigma
  )
  for (kind in names(comments_by_kind)) {
    for (comment in comments_by_kind[[kind]]) {
      if (!is.null(comment@name)) {
        if (is.null(by_user_name[[comment@name]])) {
          by_user_name[[comment@name]] <- comment
        } else {
          warning(
            "Duplicate parameter name '",
            comment@name,
            "' across parameter kinds; using first occurrence (",
            kind,
            ").",
            call. = FALSE
          )
        }
      }
    }
  }

  list(by_nonmem_name = all_comments, by_user_name = by_user_name)
}

#' @noRd
resolve_comment <- function(model_comments, nm, kind = NULL) {
  lookup_nm <- sub(" \\(.*\\)$", "", nm)

  resolve_in_kind <- function(kind_name) {
    comments <- S7::prop(model_comments, tolower(kind_name))
    comment <- comments[[lookup_nm]]
    if (!is.null(comment)) {
      return(comment)
    }
    for (cmt in comments) {
      if (!is.null(cmt@name) && identical(cmt@name, lookup_nm)) {
        return(cmt)
      }
    }
    NULL
  }

  if (!is.null(kind)) {
    kind_upper <- toupper(kind)
    if (!kind_upper %in% c("THETA", "OMEGA", "SIGMA")) {
      stop("kind must be one of: THETA, OMEGA, SIGMA")
    }
    return(resolve_in_kind(kind_upper))
  }

  matches <- list(
    THETA = resolve_in_kind("THETA"),
    OMEGA = resolve_in_kind("OMEGA"),
    SIGMA = resolve_in_kind("SIGMA")
  )
  matches <- matches[!vapply(matches, is.null, logical(1))]
  if (length(matches) > 1) {
    stop("Ambiguous parameter name '", lookup_nm, "'. Provide kind.")
  }
  if (length(matches) == 1) {
    return(matches[[1]])
  }
  NULL
}

#' Get parameterization (transform) for parameters by name
#'
#' @param model_comments A ModelComments object
#' @param names Character vector of parameter names. Can be user-defined names
#'   (e.g., "CL", "V", "OM1") or NONMEM names (e.g., "THETA1", "OMEGA(1,1)"),
#'   or a mix of both.
#' @param kind Optional character. Filter by parameter kind ("THETA", "OMEGA",
#'   or "SIGMA"). If NULL, searches all kinds.
#' @return Character vector of parameterization values (e.g., "LogNormal",
#'   "Identity", "Proportional"). Returns NA for names not found.
#' @export
get_parameter_transform <- function(model_comments, names, kind = NULL) {
  if (!S7::S7_inherits(model_comments, ModelComments)) {
    stop("model_comments must be a ModelComments object")
  }

  if (!is.null(kind)) {
    if (length(kind) != 1 && length(kind) != length(names)) {
      stop("kind must be length 1 or match length of names")
    }
    kind <- rep(kind, length.out = length(names))
  }

  # Look up each requested name
  vapply(
    seq_along(names),
    function(i) {
      nm <- names[i]
      comment <- resolve_comment(
        model_comments,
        nm,
        kind = if (!is.null(kind)) kind[i] else NULL
      )
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
#' @param kind Optional character. Filter by parameter kind ("THETA", "OMEGA",
#'   or "SIGMA"). If NULL, searches all kinds.
#' @return Character vector of unit values (e.g., "L/h", "L"). Returns NA for
#'   names not found.
#' @export
get_parameter_unit <- function(model_comments, names, kind = NULL) {
  if (!S7::S7_inherits(model_comments, ModelComments)) {
    stop("model_comments must be a ModelComments object")
  }

  if (!is.null(kind)) {
    if (length(kind) != 1 && length(kind) != length(names)) {
      stop("kind must be length 1 or match length of names")
    }
    kind <- rep(kind, length.out = length(names))
  }

  # Look up each requested name
  vapply(
    seq_along(names),
    function(i) {
      nm <- names[i]
      if (!is.null(kind) && !toupper(kind[i]) %in% c("THETA", "SIGMA")) {
        return(NA_character_)
      }
      comment <- resolve_comment(
        model_comments,
        nm,
        kind = if (!is.null(kind)) kind[i] else NULL
      )
      if (is.null(comment)) {
        return(NA_character_)
      }
      # Only theta and sigma comments have unit property
      if (
        S7::S7_inherits(comment, ThetaComment) ||
          S7::S7_inherits(comment, SigmaComment)
      ) {
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

  extract_row <- function(comment, include_associated_theta = FALSE) {
    name_val <- comment@name %||% NA_character_
    # For omega: build composite name with associated_theta (avoiding duplicates)
    if (
      include_associated_theta &&
        !is.null(comment@associated_theta) &&
        length(comment@associated_theta) > 0
    ) {
      if (is.na(name_val)) {
        name_val <- paste(comment@associated_theta, collapse = ", ")
      } else {
        name_val <- format_omega_display_name(
          name_val,
          comment@associated_theta
        )
      }
    }
    data.frame(
      name = name_val,
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
    rows <- c(
      rows,
      list(extract_row(
        model_comments@omega[[nm]],
        include_associated_theta = TRUE
      ))
    )
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
