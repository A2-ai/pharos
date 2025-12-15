VALID_PARAMETERIZATIONS <- c(
  "LogNormal",
  "Logit",
  "AddErr",
  "Proportional",
  "Identity"
)

#' Map raw parameterization string to Transform name
#'
#' @param raw_param Raw parameterization string from comment (e.g., "EXP", ":EXP")
#' @param kind Parameter kind: "THETA", "OMEGA", or "SIGMA"
#' @return Mapped parameterization name or NULL if not recognized
#' @noRd
map_parameterization <- function(raw_param, kind) {
  if (is.null(raw_param) || !nzchar(raw_param)) {
    return(NULL)
  }

  # Remove leading colon if present and convert to uppercase
  cleaned <- toupper(gsub("^:", "", trimws(raw_param)))

  # Common mappings for all parameter types
  switch(
    EXPR = cleaned,
    "EXP" = "LogNormal",
    "LOG" = "LogNormal",
    "LOGNORMAL" = "LogNormal",
    "LOGIT" = "Logit",
    "ADD" = "AddErr",
    "ADDERR" = "AddErr",
    "ADDITIVE" = "AddErr",
    "PROP" = "Proportional",
    "PROPORTIONAL" = "Proportional",
    "IDENTITY" = "Identity",
    "NORMAL" = "Identity",
    "NONE" = "Identity",
    NULL
  )
}

#' @noRd
ParameterComment <- S7::new_class(
  "ParameterComment",
  properties = list(
    nonmem_name = S7::new_property(
      S7::class_character,
      validator = function(value) {
        if (length(value) != 1 || is.na(value) || trimws(value) == "") {
          "must be a non-empty string"
        }
      }
    )
  )
)

#' Helper to create parameterization property with validation
#' @noRd
make_parameterization_property <- function() {
  S7::new_property(
    S7::class_character,
    default = "Identity",
    setter = function(self, value) {
      # Default NULL to Identity
      if (is.null(value)) {
        value <- "Identity"
      }
      if (length(value) != 1 || is.na(value)) {
        stop("@parameterization must be a single non-NA string")
      }
      matched <- match(tolower(value), tolower(VALID_PARAMETERIZATIONS))
      if (is.na(matched)) {
        stop(paste0(
          "@parameterization must be one of: ",
          paste(VALID_PARAMETERIZATIONS, collapse = ", ")
        ))
      }
      self@parameterization <- VALID_PARAMETERIZATIONS[matched]
      self
    }
  )
}

#' Theta parameter comment class
#'
#' Represents parsed comments for THETA parameters.
#'
#' @param nonmem_name Character. The NONMEM parameter name (e.g., "THETA1")
#' @param name Character or NULL. The user-defined parameter name (e.g., "CL", "V")
#' @param display Character or NULL. Display name for the parameter
#' @param description Character or NULL. Description of the parameter
#' @param unit Character or NULL. Unit of measurement (e.g., "L/hr")
#' @param parameterization Character or NULL. Transformation type. Valid values:
#'   "Log", "Exp", "Add", "Prop", "Stdev", "Corr", "OmitTbl", "Var"
#'
#' @export
ThetaComment <- S7::new_class(
  "ThetaComment",
  parent = ParameterComment,
  properties = list(
    name = S7::new_property(NULL | S7::class_character, default = NULL),
    display = S7::new_property(NULL | S7::class_character, default = NULL),
    description = S7::new_property(NULL | S7::class_character, default = NULL),
    unit = S7::new_property(NULL | S7::class_character, default = NULL),
    parameterization = make_parameterization_property()
  )
)

#' Omega parameter comment class
#'
#' Represents parsed comments for OMEGA parameters.
#'
#' @param nonmem_name Character. The NONMEM parameter name (e.g., "OMEGA(1,1)")
#' @param name Character or NULL. The user-defined parameter name (e.g., "OM1", "IIV-CL")
#' @param display Character or NULL. Display name for the parameter
#' @param description Character or NULL. Description of the parameter
#' @param parameterization Character or NULL. Transformation type. Valid values:
#'   "Log", "Exp", "Add", "Prop", "Stdev", "Corr", "OmitTbl", "Var"
#' @param associated_theta Character vector or NULL. The related theta name(s).
#'   For diagonal elements, typically a single name (e.g., "CL").
#'   For off-diagonal (covariance), multiple names (e.g., c("CL", "V")).
#'
#' @export
OmegaComment <- S7::new_class(
  "OmegaComment",
  parent = ParameterComment,
  properties = list(
    name = S7::new_property(NULL | S7::class_character, default = NULL),
    display = S7::new_property(NULL | S7::class_character, default = NULL),
    description = S7::new_property(NULL | S7::class_character, default = NULL),
    parameterization = make_parameterization_property(),
    associated_theta = S7::new_property(
      NULL | S7::class_character,
      default = NULL
    )
  )
)

#' Sigma parameter comment class
#'
#' Represents parsed comments for SIGMA parameters.
#'
#' @param nonmem_name Character. The NONMEM parameter name (e.g., "SIGMA(1,1)")
#' @param name Character or NULL. The user-defined parameter name (e.g., "SIG1", "PropErr")
#' @param display Character or NULL. Display name for the parameter
#' @param description Character or NULL. Description of the parameter
#' @param parameterization Character or NULL. Transformation type. Valid values:
#'   "Log", "Exp", "Add", "Prop", "Stdev", "Corr", "OmitTbl", "Var"
#'
#' @export
SigmaComment <- S7::new_class(
  "SigmaComment",
  parent = ParameterComment,
  properties = list(
    name = S7::new_property(NULL | S7::class_character, default = NULL),
    display = S7::new_property(NULL | S7::class_character, default = NULL),
    description = S7::new_property(NULL | S7::class_character, default = NULL),
    parameterization = make_parameterization_property()
  )
)

#' Model comments container with cross-validation
#'
#' Holds all parameter comments for a model organized by parameter type
#' (theta, omega, sigma) and validates cross-references between them.
#'
#' @param theta Named list of ThetaComment objects for THETA parameters
#' @param omega Named list of OmegaComment objects for OMEGA parameters
#' @param sigma Named list of SigmaComment objects for SIGMA parameters
#'
#' @export
ModelComments <- S7::new_class(
  "ModelComments",
  properties = list(
    theta = S7::new_property(S7::class_list, default = list()),
    omega = S7::new_property(S7::class_list, default = list()),
    sigma = S7::new_property(S7::class_list, default = list())
  ),
  validator = function(self) {
    errors <- character()

    # Type check: theta must contain ThetaComment objects
    for (name in names(self@theta)) {
      if (!S7::S7_inherits(self@theta[[name]], ThetaComment)) {
        errors <- c(
          errors,
          sprintf(
            "theta$%s must be a ThetaComment object",
            name
          )
        )
      }
    }

    # Type check: omega must contain OmegaComment objects
    for (name in names(self@omega)) {
      if (!S7::S7_inherits(self@omega[[name]], OmegaComment)) {
        errors <- c(
          errors,
          sprintf(
            "omega$%s must be a OmegaComment object",
            name
          )
        )
      }
    }

    # Type check: sigma must contain SigmaComment objects
    for (name in names(self@sigma)) {
      if (!S7::S7_inherits(self@sigma[[name]], SigmaComment)) {
        errors <- c(
          errors,
          sprintf(
            "sigma$%s must be a SigmaComment object",
            name
          )
        )
      }
    }

    # Get all theta names for cross-reference validation
    theta_names <- vapply(
      self@theta,
      function(c) if (is.null(c@name)) NA_character_ else c@name,
      character(1)
    )
    theta_names <- theta_names[!is.na(theta_names)]

    # Validate omega associated_theta references
    for (omega_name in names(self@omega)) {
      comment <- self@omega[[omega_name]]
      if (!is.null(comment@associated_theta)) {
        missing <- setdiff(comment@associated_theta, theta_names)
        if (length(missing) > 0) {
          errors <- c(
            errors,
            sprintf(
              "%s has associated_theta %s not found in theta names: %s",
              omega_name,
              paste(missing, collapse = ", "),
              paste(theta_names, collapse = ", ")
            )
          )
        }
      }
    }

    # Check for duplicate names within categories
    check_duplicates <- function(comments, category) {
      if (length(comments) == 0) return()
      names_list <- vapply(
        comments,
        function(c) if (is.null(c@name)) NA_character_ else c@name,
        character(1)
      )
      names_list <- names_list[!is.na(names_list)]
      dups <- names_list[duplicated(names_list)]
      if (length(dups) > 0) {
        errors <<- c(
          errors,
          sprintf(
            "Duplicate names in %s: %s",
            category,
            paste(unique(dups), collapse = ", ")
          )
        )
      }
    }

    check_duplicates(self@theta, "theta")
    check_duplicates(self@omega, "omega")
    check_duplicates(self@sigma, "sigma")

    if (length(errors) > 0) {
      return(paste(errors, collapse = "\n"))
    }
    NULL
  }
)

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

  # Build lookup tables: nonmem_name -> comment and user_name -> comment
  all_comments <- c(
    model_comments@theta,
    model_comments@omega,
    model_comments@sigma
  )

  # Map by nonmem_name (the list names)
  by_nonmem_name <- all_comments

  # Map by user name
  by_user_name <- list()
  for (comment in all_comments) {
    if (!is.null(comment@name)) {
      by_user_name[[comment@name]] <- comment
    }
  }

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
      comment@parameterization
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

  # Build lookup tables: nonmem_name -> comment and user_name -> comment
  all_comments <- c(
    model_comments@theta,
    model_comments@omega,
    model_comments@sigma
  )

  # Map by nonmem_name (the list names)
  by_nonmem_name <- all_comments

  # Map by user name
  by_user_name <- list()
  for (comment in all_comments) {
    if (!is.null(comment@name)) {
      by_user_name[[comment@name]] <- comment
    }
  }

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

#' Get display names for all parameters
#'
#' Creates display names for parameters from a ModelComments object.
#' For omega parameters with associated_theta, returns format "name (associated_theta)"
#' (e.g., "OM1 (KA)"). For other parameters, returns just the name.
#'
#' @param model_comments A ModelComments object
#' @return Named character vector where names are NONMEM parameter names
#'   (e.g., "THETA1", "OMEGA(1,1)") and values are display names
#'   (e.g., "CL", "OM1 (KA)")
#' @export
get_parameter_display_names <- function(model_comments, use = "name") {
  if (!S7::S7_inherits(model_comments, ModelComments)) {
    stop("model_comments must be a ModelComments object")
  }
  use <- match.arg(use, c("name", "display"))

  # Helper to get label for a single comment
  get_label <- function(comment) {
    label <- if (use == "display") comment@display else comment@name

    if (is.null(label)) {
      return(NULL)
    }

    # For omega with associated_theta, format as "label (associated_theta)"
    if (
      S7::S7_inherits(comment, OmegaComment) &&
        !is.null(comment@associated_theta)
    ) {
      return(paste0(label, " (", comment@associated_theta[1], ")"))
    }

    label
  }

  # Build named vector for each parameter type
  result <- character(0)

  for (nm in names(model_comments@theta)) {
    result[nm] <- get_label(model_comments@theta[[nm]])
  }

  for (nm in names(model_comments@omega)) {
    result[nm] <- get_label(model_comments@omega[[nm]])
  }

  for (nm in names(model_comments@sigma)) {
    result[nm] <- get_label(model_comments@sigma[[nm]])
  }

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

#' Extract all parameter comments from a model as ModelComments object
#'
#' @param mod A hyperion_nonmem_model object or path to a control stream (.mod or .ctl)
#' @param lookup_path Optional path to a yaml lookup file. If provided, fills
#'   NULL fields (display, description, unit, parameterization) from the lookup.
#' @return ModelComments object containing theta, omega, and sigma comments
#' @export
get_model_parameter_info <- function(mod, lookup_path = NULL) {
  if (is.character(mod) && length(mod) == 1) {
    mod <- read_model(mod)
  }

  if (!inherits(mod, "hyperion_nonmem_model")) {
    stop(
      "mod must be a hyperion_nonmem_model object or path to a control stream (.mod or .ctl)"
    )
  }

  param_names <- get_model_parameter_names(mod)
  comments_data <- extract_comments(mod)
  comments <- parse_comments(
    param_names,
    comments_data$parsed,
    comments_data$raw
  )

  if (!is.null(lookup_path)) {
    for (name in names(comments)) {
      comments[[name]] <- apply_lookup_defaults(comments[[name]], lookup_path)
    }
  }

  # Split into theta, omega, sigma
  theta_comments <- comments[grepl("^THETA", names(comments))]
  omega_comments <- comments[grepl("^OMEGA", names(comments))]
  sigma_comments <- comments[grepl("^SIGMA", names(comments))]

  # Create ModelComments object with validation
  ModelComments(
    theta = theta_comments,
    omega = omega_comments,
    sigma = sigma_comments
  )
}

#' @noRd
extract_comments <- function(mod) {
  parsed <- list()
  raw <- list()

  for (i in seq_along(mod$theta_parameters)) {
    old_name <- paste0("THETA", i)
    parsed[[old_name]] <- mod$theta_parameters[[i]]$parsed_comment
    raw[[old_name]] <- mod$theta_parameters[[i]]$comment
  }

  result <- extract_block_comments(parsed, raw, mod$omega_blocks, "OMEGA")
  parsed <- result$parsed
  raw <- result$raw

  result <- extract_block_comments(parsed, raw, mod$sigma_blocks, "SIGMA")

  list(parsed = result$parsed, raw = result$raw)
}

#' @noRd
extract_block_comments <- function(parsed, raw, blocks, prefix) {
  row <- 1

  for (block in blocks) {
    struct <- block$structure

    # Handle structure as string "Diagonal" or list with named element
    is_diagonal <- identical(struct, "Diagonal") ||
      (is.list(struct) && "Diagonal" %in% names(struct))
    is_block <- is.list(struct) && "Block" %in% names(struct)
    is_block_same <- is.list(struct) && "BlockSame" %in% names(struct)

    if (is_diagonal) {
      for (param in block$parameters) {
        old_name <- sprintf("%s(%d,%d)", prefix, row, row)
        parsed[[old_name]] <- param$parsed_comment
        raw[[old_name]] <- param$comment
        row <- row + 1
      }
    } else if (is_block) {
      block_size <- struct$Block$size
      param_idx <- 1
      start_row <- row

      for (i in seq_len(block_size)) {
        for (j in seq_len(i)) {
          old_name <- sprintf(
            "%s(%d,%d)",
            prefix,
            start_row + i - 1,
            start_row + j - 1
          )
          # Only diagonal elements (i == j) get the comment
          # Off-diagonal elements share a line but comment applies to diagonal only
          if (i == j) {
            parsed[[old_name]] <- block$parameters[[param_idx]]$parsed_comment
            raw[[old_name]] <- block$parameters[[param_idx]]$comment
          } else {
            parsed[[old_name]] <- NULL
            raw[[old_name]] <- NULL
          }
          param_idx <- param_idx + 1
        }
      }
      row <- start_row + block_size
    } else if (is_block_same) {
      block_size <- struct$BlockSame$size
      row <- row + block_size
    }
  }

  list(parsed = parsed, raw = raw)
}

#' Parse comments from model based on comment_type setting
#' @noRd
parse_comments <- function(param_names, parsed_comments, raw_comments) {
  comment_type <- get_comment_type()

  if (identical(comment_type, "type1")) {
    parse_type1_comments(param_names, parsed_comments, raw_comments)
  } else {
    parse_raw_comments(param_names, raw_comments)
  }
}

# ==============================================================================
# Raw comment parsing (no comment_type set, extract from raw text only)
# ==============================================================================

#' @noRd
parse_raw_comments <- function(param_names, raw_comments) {
  nonmem_names <- names(param_names)
  comments <- lapply(nonmem_names, function(nonmem_name) {
    name <- param_names[[nonmem_name]]
    raw <- raw_comments[[nonmem_name]]

    if (grepl("^THETA", nonmem_name)) {
      parse_raw_theta_comment(nonmem_name, name, raw)
    } else if (grepl("^OMEGA", nonmem_name)) {
      parse_raw_omega_comment(nonmem_name, name, raw)
    } else if (grepl("^SIGMA", nonmem_name)) {
      parse_raw_sigma_comment(nonmem_name, name, raw)
    } else {
      stop("Unknown parameter type: ", nonmem_name)
    }
  })
  names(comments) <- nonmem_names
  comments
}

#' @noRd
parse_raw_theta_comment <- function(nonmem_name, name, raw) {
  if (!is.null(name) && (!nzchar(name) || is.na(name))) {
    name <- NULL
  }

  unit <- NULL
  parameterization <- NULL

  if (!is.null(raw) && nzchar(raw)) {
    parts <- extract_raw_theta_parts(raw)
    if (is.null(name)) name <- parts$name
    unit <- parts$unit
    parameterization <- map_parameterization(parts$parameterization, "THETA")
  }

  ThetaComment(
    nonmem_name = nonmem_name,
    name = name,
    unit = unit,
    parameterization = parameterization
  )
}

#' @noRd
parse_raw_omega_comment <- function(nonmem_name, name, raw) {
  if (!is.null(name) && (!nzchar(name) || is.na(name))) {
    name <- NULL
  }

  parameterization <- NULL
  associated_theta <- NULL

  if (!is.null(raw) && nzchar(raw)) {
    parts <- extract_raw_omega_parts(raw)
    if (is.null(name)) name <- parts$name
    parameterization <- map_parameterization(parts$parameterization, "OMEGA")
    associated_theta <- parts$associated_theta
  }

  OmegaComment(
    nonmem_name = nonmem_name,
    name = name,
    parameterization = parameterization,
    associated_theta = associated_theta
  )
}

#' @noRd
parse_raw_sigma_comment <- function(nonmem_name, name, raw) {
  if (!is.null(name) && (!nzchar(name) || is.na(name))) {
    name <- NULL
  }

  parameterization <- NULL

  if (!is.null(raw) && nzchar(raw)) {
    parts <- extract_raw_sigma_parts(raw)
    if (is.null(name)) name <- parts$name
    parameterization <- map_parameterization(parts$parameterization, "SIGMA")
  }

  SigmaComment(
    nonmem_name = nonmem_name,
    name = name,
    parameterization = parameterization
  )
}

# ==============================================================================
# Type1 comment parsing
# ==============================================================================

#' @noRd
parse_type1_comments <- function(param_names, parsed_comments, raw_comments) {
  nonmem_names <- names(param_names)
  comments <- lapply(nonmem_names, function(nonmem_name) {
    name <- param_names[[nonmem_name]]
    parsed <- parsed_comments[[nonmem_name]]
    raw <- raw_comments[[nonmem_name]]

    if (grepl("^THETA", nonmem_name)) {
      parse_type1_theta_comment(nonmem_name, name, parsed, raw)
    } else if (grepl("^OMEGA", nonmem_name)) {
      parse_type1_omega_comment(nonmem_name, name, parsed, raw)
    } else if (grepl("^SIGMA", nonmem_name)) {
      parse_type1_sigma_comment(nonmem_name, name, parsed, raw)
    } else {
      stop("Unknown parameter type: ", nonmem_name)
    }
  })
  names(comments) <- nonmem_names
  comments
}

#' @noRd
parse_type1_theta_comment <- function(nonmem_name, name, parsed, raw) {
  # Convert empty string to NULL
  if (!is.null(name) && (!nzchar(name) || is.na(name))) {
    name <- NULL
  }

  unit <- NULL
  parameterization <- NULL

  # Try to extract from parsed comment
  if (!is.null(parsed) && !is.null(parsed$Type1)) {
    type1 <- parsed$Type1

    if (!is.null(type1$WithUnit)) {
      if (is.null(name)) name <- type1$WithUnit$parameter
      if (is.null(unit)) unit <- type1$WithUnit$unit
      if (is.null(parameterization))
        parameterization <- map_parameterization(
          type1$WithUnit$parametrization,
          "THETA"
        )
    } else if (!is.null(type1$Type)) {
      if (is.null(name)) name <- type1$Type$typ
      if (is.null(parameterization))
        parameterization <- map_parameterization(
          type1$Type$parameterization,
          "THETA"
        )
    } else if (!is.null(type1$Covariate)) {
      if (is.null(name)) name <- type1$Covariate$parameter
    } else if (is.character(type1)) {
      if (is.null(name)) name <- extract_name_from_raw(type1)
    }
  }

  # Fallback: extract from raw comment
  if (is.null(name) && !is.null(raw) && nzchar(raw)) {
    name <- extract_name_from_raw(raw)
  }

  ThetaComment(
    nonmem_name = nonmem_name,
    name = name,
    unit = unit,
    parameterization = parameterization
  )
}

#' Check if an omega parameter is diagonal (variance) vs off-diagonal (covariance)
#' @noRd
is_diagonal_omega <- function(nonmem_name) {
  # Parse OMEGA(i,j) format
  match <- regmatches(
    nonmem_name,
    regexec("OMEGA\\((\\d+),(\\d+)\\)", nonmem_name)
  )[[1]]
  if (length(match) == 3) {
    return(match[2] == match[3])
  }
  # If we can't parse, assume diagonal
  TRUE
}

#' @noRd
parse_type1_omega_comment <- function(nonmem_name, name, parsed, raw) {
  # Convert empty string to NULL
  if (!is.null(name) && (!nzchar(name) || is.na(name))) {
    name <- NULL
  }

  parameterization <- NULL
  associated_theta <- NULL

  # Check if this is a diagonal element - associated_theta only applies to diagonal
  is_diagonal <- is_diagonal_omega(nonmem_name)

  # Parse name format: "OM1 (CL)" to extract associated_theta (diagonal only)
  if (is_diagonal && !is.null(name) && grepl("\\(.*\\)", name)) {
    associated_theta <- gsub(".*\\((.+)\\).*", "\\1", name)
    name <- trimws(gsub("\\s*\\(.*\\)\\s*$", "", name))
  }

  # Try to extract from parsed comment
  if (!is.null(parsed) && !is.null(parsed$Type1)) {
    type1 <- parsed$Type1

    if (is.character(type1)) {
      # Type1$Unknown: raw string stored directly
      if (is.null(name)) {
        parsed_raw <- extract_raw_omega_parts(type1)
        name <- parsed_raw$name
        if (is_diagonal && is.null(associated_theta))
          associated_theta <- parsed_raw$associated_theta
        if (is.null(parameterization))
          parameterization <- map_parameterization(
            parsed_raw$parameterization,
            "OMEGA"
          )
      }
    } else {
      # Omega style: name, theta_name, parameterization
      if (is.null(name)) name <- type1$name
      if (is_diagonal && is.null(associated_theta))
        associated_theta <- type1$theta_name
      if (is.null(parameterization))
        parameterization <- map_parameterization(
          type1$parameterization,
          "OMEGA"
        )
    }
  }

  # Fallback: extract from raw comment (diagonal only for associated_theta)
  if (
    (is.null(name) || (is_diagonal && is.null(associated_theta))) &&
      !is.null(raw) &&
      nzchar(raw)
  ) {
    parsed_raw <- extract_raw_omega_parts(raw)
    if (is.null(name)) name <- parsed_raw$name
    if (is_diagonal && is.null(associated_theta))
      associated_theta <- parsed_raw$associated_theta
    if (is.null(parameterization))
      parameterization <- map_parameterization(
        parsed_raw$parameterization,
        "OMEGA"
      )
  }

  OmegaComment(
    nonmem_name = nonmem_name,
    name = name,
    parameterization = parameterization,
    associated_theta = associated_theta
  )
}

#' @noRd
parse_type1_sigma_comment <- function(nonmem_name, name, parsed, raw) {
  # Convert empty string to NULL
  if (!is.null(name) && (!nzchar(name) || is.na(name))) {
    name <- NULL
  }

  parameterization <- NULL

  # Try to extract from parsed comment
  if (!is.null(parsed) && !is.null(parsed$Type1)) {
    type1 <- parsed$Type1

    if (is.character(type1)) {
      # Type1$Unknown: raw string stored directly
      if (is.null(name)) {
        parsed_raw <- extract_raw_sigma_parts(type1)
        name <- parsed_raw$name
        if (is.null(parameterization))
          parameterization <- map_parameterization(
            parsed_raw$parameterization,
            "SIGMA"
          )
      }
    } else {
      # Sigma style: name, parameterization
      if (is.null(name)) name <- type1$name
      if (is.null(parameterization))
        parameterization <- map_parameterization(
          type1$parameterization,
          "SIGMA"
        )
    }
  }

  # Fallback: extract from raw comment
  if (is.null(name) && !is.null(raw) && nzchar(raw)) {
    parsed_raw <- extract_raw_sigma_parts(raw)
    name <- parsed_raw$name
    if (is.null(parameterization))
      parameterization <- map_parameterization(
        parsed_raw$parameterization,
        "SIGMA"
      )
  }

  SigmaComment(
    nonmem_name = nonmem_name,
    name = name,
    parameterization = parameterization
  )
}

#' Extract name from raw comment string
#'
#' Finds the first alphanumeric word, skipping leading pure numbers.
#'
#' @param raw Character string of the raw comment
#' @return Character string of the extracted name, or NULL if none found
#' @noRd
extract_name_from_raw <- function(raw) {
  if (is.null(raw) || !nzchar(trimws(raw))) {
    return(NULL)
  }

  words <- strsplit(trimws(raw), "\\s+")[[1]]
  idx <- find_first_name_idx(words)

  if (!is.na(idx)) {
    return(words[idx])
  }

  NULL
}

#' Extract parameterization suffix from raw comment
#'
#' Handles formats: "; exp", ";exp", " :EXP"
#'
#' @param raw Character string of the raw comment
#' @return Named list with remaining raw string and parameterization
#' @noRd
extract_parameterization_suffix <- function(raw) {
  parameterization <- NULL

  if (grepl(";", raw)) {
    parts <- strsplit(raw, ";")[[1]]
    raw <- trimws(parts[1])
    if (length(parts) >= 2) {
      param_part <- trimws(parts[2])
      if (nzchar(param_part)) {
        parameterization <- param_part
      }
    }
  } else if (grepl("\\s+:[A-Za-z]+\\s*$", raw)) {
    match <- regmatches(raw, regexec("\\s+:([A-Za-z]+)\\s*$", raw))[[1]]
    if (length(match) >= 2) {
      parameterization <- match[2]
      raw <- trimws(sub("\\s+:[A-Za-z]+\\s*$", "", raw))
    }
  }

  list(raw = raw, parameterization = parameterization)
}

#' Strip parameter prefix from raw comment
#'
#' Removes THETAn:, OMEGAn:, OMEGA(n,n):, SIGMAn:, SIGMA(n,n): prefixes
#'
#' @param raw Character string of the raw comment
#' @return Character string with prefix removed
#' @noRd
strip_param_prefix <- function(raw) {
  raw <- gsub("^THETA\\d+:\\s*", "", raw)
  raw <- gsub("^OMEGA\\d+:\\s*", "", raw)
  raw <- gsub("^OMEGA\\(\\d+,\\d+\\):\\s*", "", raw)
  raw <- gsub("^SIGMA\\d+:\\s*", "", raw)
  raw <- gsub("^SIGMA\\(\\d+,\\d+\\):\\s*", "", raw)
  raw
}

#' Find first word containing letters
#'
#' @param words Character vector of words
#' @return Index of first word with letters, or NA if none found
#' @noRd
find_first_name_idx <- function(words) {
  for (i in seq_along(words)) {
    if (grepl("[A-Za-z]", words[i])) {
      return(i)
    }
  }
  NA_integer_
}

#' Extract components from raw theta comment string
#'
#' Parses comments like "THETA1: CL (L/day) ; exp" or "CL (L/day)"
#'
#' @param raw Character string of the raw comment
#' @return Named list with name, unit, and parameterization
#' @noRd
extract_raw_theta_parts <- function(raw) {
  result <- list(name = NULL, unit = NULL, parameterization = NULL)

  if (is.null(raw) || !nzchar(trimws(raw))) {
    return(result)
  }

  raw <- trimws(raw)

  # Extract parameterization suffix
  extracted <- extract_parameterization_suffix(raw)
  raw <- extracted$raw
  result$parameterization <- extracted$parameterization

  # Strip parameter prefix
  raw <- strip_param_prefix(raw)

  # Extract unit from parentheses (e.g., "CL (L/day)" -> unit="L/day")
  if (grepl("\\([^)]+\\)", raw)) {
    unit_match <- regmatches(raw, regexec("\\(([^)]+)\\)", raw))[[1]]
    if (length(unit_match) >= 2) {
      result$unit <- unit_match[2]
    }
    raw <- trimws(gsub("\\s*\\([^)]+\\)", "", raw))
  }

  # Find name (first word with letters)
  if (nzchar(raw)) {
    words <- strsplit(raw, "\\s+")[[1]]
    idx <- find_first_name_idx(words)
    if (!is.na(idx)) {
      result$name <- words[idx]
    }
  }

  result
}

#' Extract components from raw omega comment string
#'
#' Parses comments like "OM1  CL", "OM2,1 CL-VC", "OM1 CL :EXP", or "OMEGA1: CL ; exp"
#'
#' @param raw Character string of the raw comment
#' @return Named list with name, associated_theta (character vector), and parameterization
#' @noRd
extract_raw_omega_parts <- function(raw) {
  result <- list(name = NULL, associated_theta = NULL, parameterization = NULL)

  if (is.null(raw) || !nzchar(trimws(raw))) {
    return(result)
  }

  raw <- trimws(raw)

  # Extract parameterization suffix
  extracted <- extract_parameterization_suffix(raw)
  raw <- extracted$raw
  result$parameterization <- extracted$parameterization

  # Strip parameter prefix
  raw <- strip_param_prefix(raw)

  # Split remaining into words and find name
  words <- strsplit(raw, "\\s+")[[1]]
  idx <- find_first_name_idx(words)

  if (!is.na(idx)) {
    result$name <- words[idx]

    # Next word is the theta reference, may contain "-", "/", ":" or "," for covariance
    if (idx + 1 <= length(words)) {
      theta_part <- words[idx + 1]
      if (grepl("[-/:,]", theta_part)) {
        result$associated_theta <- strsplit(theta_part, "[-/:,]")[[1]]
      } else {
        result$associated_theta <- theta_part
      }
    }
  }

  result
}

#' Extract components from raw sigma comment string
#'
#' Parses comments like "SIG1", "PropErr", "AddErr :PROP", or "SIGMA1: PropErr ; prop"
#' Returns NULL for name if comment is a numbered description (e.g., "1. Proportional error...")
#'
#' @param raw Character string of the raw comment
#' @return Named list with name and parameterization
#' @noRd
extract_raw_sigma_parts <- function(raw) {
  result <- list(name = NULL, parameterization = NULL)

  if (is.null(raw) || !nzchar(trimws(raw))) {
    return(result)
  }

  raw <- trimws(raw)

  # Check if this is a numbered description (e.g., "1. Proportional error..." or "2 Additive...")
  # These are descriptions, not names - return NULL for name
  if (grepl("^\\d+\\.?\\s", raw)) {
    return(result)
  }

  # Extract parameterization suffix using shared helper
  extracted <- extract_parameterization_suffix(raw)
  raw <- extracted$raw
  result$parameterization <- extracted$parameterization

  # Strip parameter prefix using shared helper
  raw <- strip_param_prefix(raw)

  # Find name (first word with letters) using shared helper
  words <- strsplit(raw, "\\s+")[[1]]
  idx <- find_first_name_idx(words)
  if (!is.na(idx)) {
    result$name <- words[idx]
  }

  result
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

  if (is.null(comment@name)) {
    return(comment)
  }

  lookup <- load_lookup_yaml(lookup_path)

  if (!comment@name %in% names(lookup)) {
    return(comment)
  }

  entry <- lookup[[comment@name]]

  if (is.null(comment@display) && !is.null(entry$display)) {
    comment@display <- entry$display
  }

  if (is.null(comment@description) && !is.null(entry$desc)) {
    comment@description <- entry$desc
  }

  # Only theta has unit property
  if (is_theta && is.null(comment@unit) && !is.null(entry$unit)) {
    resolved_unit <- resolve_unit(entry$unit, lookup)
    if (!is.null(resolved_unit) && resolved_unit != "none") {
      comment@unit <- resolved_unit
    }
  }

  if (is.null(comment@parameterization) && !is.null(entry$parameterization)) {
    if (entry$parameterization != "none") {
      comment@parameterization <- entry$parameterization
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
