VALID_PARAMETERIZATIONS <- c(
  "LogNormal",
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
    cleaned,
    "EXP" = "LogNormal",
    "LOG" = "LogNormal",
    "LOGNORMAL" = "LogNormal",
    "ADD" = "AddErr",
    "ADDERR" = "AddErr",
    "ADDITIVE" = "AddErr",
    "PROP" = "Proportional",
    "PROPORTIONAL" = "Proportional",
    "IDENTITY" = "Identity",
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

#' Type1 theta parameter comment class
#'
#' Represents Type1 format comments for THETA parameters.
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
Type1ThetaComment <- S7::new_class(
  "Type1ThetaComment",
  parent = ParameterComment,
  properties = list(
    name = S7::new_property(NULL | S7::class_character, default = NULL),
    display = S7::new_property(NULL | S7::class_character, default = NULL),
    description = S7::new_property(NULL | S7::class_character, default = NULL),
    unit = S7::new_property(NULL | S7::class_character, default = NULL),
    parameterization = make_parameterization_property()
  )
)

#' Type1 omega parameter comment class
#'
#' Represents Type1 format comments for OMEGA parameters.
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
Type1OmegaComment <- S7::new_class(
  "Type1OmegaComment",
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

#' Type1 sigma parameter comment class
#'
#' Represents Type1 format comments for SIGMA parameters.
#'
#' @param nonmem_name Character. The NONMEM parameter name (e.g., "SIGMA(1,1)")
#' @param name Character or NULL. The user-defined parameter name (e.g., "SIG1", "PropErr")
#' @param display Character or NULL. Display name for the parameter
#' @param description Character or NULL. Description of the parameter
#' @param parameterization Character or NULL. Transformation type. Valid values:
#'   "Log", "Exp", "Add", "Prop", "Stdev", "Corr", "OmitTbl", "Var"
#'
#' @export
Type1SigmaComment <- S7::new_class(
  "Type1SigmaComment",
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
#' @param theta Named list of Type1ThetaComment objects for THETA parameters
#' @param omega Named list of Type1OmegaComment objects for OMEGA parameters
#' @param sigma Named list of Type1SigmaComment objects for SIGMA parameters
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

    # Type check: theta must contain Type1ThetaComment objects
    for (name in names(self@theta)) {
      if (!S7::S7_inherits(self@theta[[name]], Type1ThetaComment)) {
        errors <- c(
          errors,
          sprintf(
            "theta$%s must be a Type1ThetaComment object",
            name
          )
        )
      }
    }

    # Type check: omega must contain Type1OmegaComment objects
    for (name in names(self@omega)) {
      if (!S7::S7_inherits(self@omega[[name]], Type1OmegaComment)) {
        errors <- c(
          errors,
          sprintf(
            "omega$%s must be a Type1OmegaComment object",
            name
          )
        )
      }
    }

    # Type check: sigma must contain Type1SigmaComment objects
    for (name in names(self@sigma)) {
      if (!S7::S7_inherits(self@sigma[[name]], Type1SigmaComment)) {
        errors <- c(
          errors,
          sprintf(
            "sigma$%s must be a Type1SigmaComment object",
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
#' @return The Type1Comment object, or NULL if not found
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
      if (S7::S7_inherits(comment, Type1ThetaComment)) {
        return(comment@unit %||% NA_character_)
      }
      NA_character_
    },
    character(1),
    USE.NAMES = FALSE
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
  comments <- comments_from_hybrid(
    mod,
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
          parsed[[old_name]] <- block$parameters[[param_idx]]$parsed_comment
          raw[[old_name]] <- block$parameters[[param_idx]]$comment
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

#' @noRd
comments_from_hybrid <- function(
  mod,
  param_names,
  parsed_comments,
  raw_comments
) {
  # Get Comment type from pharos.toml
  comment_type <- get_comment_type()

  if (is.null(comment_type)) {
    stop("comment_type not set in pharos.toml")
  }

  if (comment_type != "type1") {
    stop("Unknown comment type: ", comment_type)
  }

  nonmem_names <- names(param_names)
  comments <- lapply(nonmem_names, function(nonmem_name) {
    name <- param_names[[nonmem_name]]
    parsed <- parsed_comments[[nonmem_name]]
    raw <- raw_comments[[nonmem_name]]

    # Dispatch to appropriate factory based on parameter type
    if (grepl("^THETA", nonmem_name)) {
      type1_theta_from_hybrid(nonmem_name, name, parsed, raw)
    } else if (grepl("^OMEGA", nonmem_name)) {
      type1_omega_from_hybrid(nonmem_name, name, parsed, raw)
    } else if (grepl("^SIGMA", nonmem_name)) {
      type1_sigma_from_hybrid(nonmem_name, name, parsed, raw)
    } else {
      stop("Unknown parameter type: ", nonmem_name)
    }
  })
  names(comments) <- nonmem_names
  comments
}

#' @noRd
type1_theta_from_hybrid <- function(nonmem_name, name, parsed, raw) {
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

  Type1ThetaComment(
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
type1_omega_from_hybrid <- function(nonmem_name, name, parsed, raw) {
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
        parsed_raw <- parse_raw_omega_comment(type1)
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
    parsed_raw <- parse_raw_omega_comment(raw)
    if (is.null(name)) name <- parsed_raw$name
    if (is_diagonal && is.null(associated_theta))
      associated_theta <- parsed_raw$associated_theta
    if (is.null(parameterization))
      parameterization <- map_parameterization(
        parsed_raw$parameterization,
        "OMEGA"
      )
  }

  Type1OmegaComment(
    nonmem_name = nonmem_name,
    name = name,
    parameterization = parameterization,
    associated_theta = associated_theta
  )
}

#' @noRd
type1_sigma_from_hybrid <- function(nonmem_name, name, parsed, raw) {
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
        parsed_raw <- parse_raw_sigma_comment(type1)
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
    parsed_raw <- parse_raw_sigma_comment(raw)
    name <- parsed_raw$name
    if (is.null(parameterization))
      parameterization <- map_parameterization(
        parsed_raw$parameterization,
        "SIGMA"
      )
  }

  Type1SigmaComment(
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

  # Split into words
  words <- strsplit(trimws(raw), "\\s+")[[1]]

  # Find first word that contains at least one letter (not pure number)
  for (word in words) {
    if (grepl("[A-Za-z]", word)) {
      return(word)
    }
  }

  NULL
}

#' Parse raw omega comment to extract components
#'
#' Parses comments like "OM1  CL", "OM2,1 CL-VC", or "OM1 CL :EXP"
#'
#' @param raw Character string of the raw comment
#' @return Named list with name, associated_theta (character vector), and parameterization
#' @noRd
parse_raw_omega_comment <- function(raw) {
  result <- list(name = NULL, associated_theta = NULL, parameterization = NULL)

  if (is.null(raw) || !nzchar(trimws(raw))) {
    return(result)
  }

  raw <- trimws(raw)

  # Check for parameterization suffix first (e.g., ":EXP")
  if (grepl(":", raw)) {
    parts <- strsplit(raw, ":")[[1]]
    raw <- trimws(parts[1])
    param_part <- trimws(parts[2])
    if (nzchar(param_part)) {
      result$parameterization <- param_part
    }
  }

  # Split remaining into words
  words <- strsplit(raw, "\\s+")[[1]]

  if (length(words) >= 1) {
    # First word is the name (e.g., "OM1", "OM2,1")
    result$name <- words[1]
  }

  if (length(words) >= 2) {
    # Second word is the theta reference, may contain "-" for covariance
    theta_part <- words[2]
    if (grepl("-", theta_part)) {
      # Split by "-" for covariance terms like "CL-VC"
      result$associated_theta <- strsplit(theta_part, "-")[[1]]
    } else {
      result$associated_theta <- theta_part
    }
  }

  result
}

#' Parse raw sigma comment to extract components
#'
#' Parses comments like "SIG1", "PropErr", or "AddErr :PROP"
#' Returns NULL for name if comment is a numbered description (e.g., "1. Proportional error...")
#'
#' @param raw Character string of the raw comment
#' @return Named list with name and parameterization
#' @noRd
parse_raw_sigma_comment <- function(raw) {
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

  # Check for parameterization suffix first (e.g., ":PROP")
  if (grepl(":", raw)) {
    parts <- strsplit(raw, ":")[[1]]
    raw <- trimws(parts[1])
    param_part <- trimws(parts[2])
    if (nzchar(param_part)) {
      result$parameterization <- param_part
    }
  }

  # Split remaining into words
  words <- strsplit(raw, "\\s+")[[1]]

  if (length(words) >= 1) {
    # First word is the name (e.g., "SIG1", "PropErr")
    result$name <- words[1]
  }

  result
}

#' Apply lookup defaults to a parameter comment
#'
#' Fills NULL fields (display, description, unit, parameterization) from a
#' lookup yaml file. Matches the comment's `name` field against yaml keys.
#'
#' @param comment A Type1ThetaComment, Type1OmegaComment, or Type1SigmaComment object
#' @param lookup_path Path to a yaml lookup file
#' @return The modified comment object
#' @export
apply_lookup_defaults <- function(comment, lookup_path) {
  is_theta <- S7::S7_inherits(comment, Type1ThetaComment)
  is_omega <- S7::S7_inherits(comment, Type1OmegaComment)
  is_sigma <- S7::S7_inherits(comment, Type1SigmaComment)

  if (!is_theta && !is_omega && !is_sigma) {
    stop(
      "comment must be a Type1ThetaComment, Type1OmegaComment, or Type1SigmaComment object"
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
