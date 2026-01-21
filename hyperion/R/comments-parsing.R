#' Make a path relative to project root (pharos.toml directory)
#' @noRd
relative_path <- function(path) {
  if (is.null(path) || path == "default" || path == "user supplied") {
    return(path)
  }
  tryCatch(
    {
      config_path <- find_pharos_config_file()
      if (grepl("No pharos.toml", config_path)) {
        return(path)
      }
      root <- fs::path_dir(config_path)
      as.character(fs::path_rel(path, start = root))
    },
    error = function(e) path
  )
}

#' Set source paths for comment fields
#'
#' Always initializes the sources attribute to mark object as "initialized".
#' Fields with non-NULL values get source_path; NULL fields get "default".
#' @noRd
set_sources <- function(comment, fields, source_path) {
  source_path <- relative_path(source_path)
  sources <- list()
  for (f in fields) {
    val <- S7::prop(comment, f)
    if (!is.null(val)) {
      sources[[f]] <- source_path
    } else {
      sources[[f]] <- "default"
    }
  }
  attr(comment, "sources") <- sources
  comment
}

#' @noRd
normalize_comment_name <- function(name) {
  if (!is.null(name) && (!nzchar(name) || is.na(name))) {
    return(NULL)
  }
  name
}

#' @noRd
create_comment_with_sources <- function(constructor, fields, mod_path, ...) {
  comment <- constructor(...)
  set_sources(comment, fields, mod_path)
}

#' Extract all parameter comments from a model as ModelComments object
#'
#' @param mod A hyperion_nonmem_model object or path to a run output directory containing an .lst file
#' @param lookup_path Optional path to a toml lookup file. If provided, fills
#'   NULL fields (display, description, unit, parameterization) from the lookup.
#' @return ModelComments object containing theta, omega, and sigma comments
#' @export
get_model_parameter_info <- function(mod, lookup_path = NULL) {
  if (is.character(mod) && length(mod) == 1) {
    mod_path <- normalizePath(mod, mustWork = FALSE)
    if (!dir.exists(mod_path)) {
      stop(
        "mod must be a run output directory containing an .lst file: ",
        mod_path
      )
    }
    lst_candidates <- list.files(
      mod_path,
      pattern = "\\.lst$",
      ignore.case = TRUE,
      full.names = TRUE
    )
    if (length(lst_candidates) == 0) {
      stop("lst file not found in run directory: ", mod_path)
    }
    lst_path <- lst_candidates[1]
    mod <- read_model_from_lst(lst_path)
  } else if (inherits(mod, "hyperion_nonmem_model")) {
    mod_path <- attr(mod, "model_source") %||% "unknown"
  } else {
    stop(
      "mod must be a hyperion_nonmem_model object or path to a run output directory containing an .lst file"
    )
  }

  run_status <- attr(mod, "run_status") %||% NA_character_
  if (!identical(run_status, "run")) {
    stop("model run_status must be run, got: ", run_status)
  }

  mod_path <- attr(mod, "model_source") %||% "unknown"
  if (!grepl("\\.lst$", mod_path, ignore.case = TRUE)) {
    warning("model_source is not an .lst file: ", mod_path, call. = FALSE)
  }

  param_names <- get_model_parameter_names(mod)
  comments_data <- extract_comments(mod)
  comments <- parse_comments(
    param_names,
    comments_data$parsed,
    comments_data$raw,
    mod_path
  )

  if (!is.null(lookup_path)) {
    lookup_path <- normalizePath(lookup_path, mustWork = FALSE)
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
        # Track elements on this row
        row_names <- character(i)

        for (j in seq_len(i)) {
          old_name <- sprintf(
            "%s(%d,%d)",
            prefix,
            start_row + i - 1,
            start_row + j - 1
          )
          row_names[j] <- old_name
          parsed[[old_name]] <- block$parameters[[param_idx]]$parsed_comment
          raw[[old_name]] <- block$parameters[[param_idx]]$comment
          param_idx <- param_idx + 1
        }

        # Clear duplicate comments from off-diagonals
        # (when elements share a source line, they all get the same comment from parser)
        if (i > 1) {
          diag_name <- row_names[i] # Last element is diagonal (j == i)
          diag_comment <- raw[[diag_name]]
          if (!is.null(diag_comment) && nzchar(diag_comment)) {
            for (k in seq_len(i - 1)) {
              off_diag_name <- row_names[k]
              if (identical(raw[[off_diag_name]], diag_comment)) {
                raw[[off_diag_name]] <- NULL
                parsed[[off_diag_name]] <- NULL
              }
            }
          }
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
parse_comments <- function(
  param_names,
  parsed_comments,
  raw_comments,
  mod_path
) {
  comment_type <- get_comment_type()

  if (identical(comment_type, "type1")) {
    parse_type1_comments(param_names, parsed_comments, raw_comments, mod_path)
  } else {
    parse_raw_comments(param_names, raw_comments, mod_path)
  }
}

# ==============================================================================
# Raw comment parsing (no comment_type set, extract from raw text only)
# ==============================================================================

#' @noRd
parse_raw_comments <- function(param_names, raw_comments, mod_path) {
  nonmem_names <- names(param_names)

  # First pass: parse thetas to collect known theta names
  theta_names <- nonmem_names[grepl("^THETA", nonmem_names)]
  theta_comments <- lapply(theta_names, function(nonmem_name) {
    name <- param_names[[nonmem_name]]
    raw <- raw_comments[[nonmem_name]]
    parse_raw_theta_comment(nonmem_name, name, raw, mod_path)
  })
  names(theta_comments) <- theta_names

  # Collect known theta names for context
  known_thetas <- vapply(
    theta_comments,
    function(c) c@name %||% "",
    character(1)
  )
  known_thetas <- known_thetas[nzchar(known_thetas)]

  # Second pass: parse omega/sigma with known_thetas context
  other_names <- nonmem_names[!grepl("^THETA", nonmem_names)]
  other_comments <- lapply(other_names, function(nonmem_name) {
    name <- param_names[[nonmem_name]]
    raw <- raw_comments[[nonmem_name]]

    if (grepl("^OMEGA", nonmem_name)) {
      parse_raw_omega_comment(nonmem_name, name, raw, mod_path, known_thetas)
    } else if (grepl("^SIGMA", nonmem_name)) {
      parse_raw_sigma_comment(nonmem_name, name, raw, mod_path)
    } else {
      stop("Unknown parameter type: ", nonmem_name)
    }
  })
  names(other_comments) <- other_names

  # Combine and preserve original order
  comments <- c(theta_comments, other_comments)
  comments[nonmem_names]
}

#' @noRd
parse_raw_theta_comment <- function(nonmem_name, name, raw, mod_path = NULL) {
  name <- normalize_comment_name(name)

  unit <- NULL
  parameterization <- NULL

  if (!is.null(raw) && nzchar(raw)) {
    parts <- extract_raw_theta_parts(raw)
    if (is.null(name)) name <- parts$name
    unit <- parts$unit
    parameterization <- map_parameterization(parts$parameterization, "THETA")
  }

  create_comment_with_sources(
    ThetaComment,
    theta_fields(),
    mod_path,
    nonmem_name = nonmem_name,
    name = name,
    unit = unit,
    parameterization = parameterization
  )
}

#' @noRd
parse_raw_omega_comment <- function(
  nonmem_name,
  name,
  raw,
  mod_path = NULL,
  known_thetas = NULL
) {
  name <- normalize_comment_name(name)

  parameterization <- NULL
  associated_theta <- NULL

  if (!is.null(raw) && nzchar(raw)) {
    parts <- extract_raw_omega_parts(raw, known_thetas)
    if (is.null(name)) name <- parts$name
    parameterization <- map_parameterization(parts$parameterization, "OMEGA")
    associated_theta <- parts$associated_theta
  }

  create_comment_with_sources(
    OmegaComment,
    omega_fields(),
    mod_path,
    nonmem_name = nonmem_name,
    name = name,
    parameterization = parameterization,
    associated_theta = associated_theta
  )
}

#' @noRd
parse_raw_sigma_comment <- function(nonmem_name, name, raw, mod_path = NULL) {
  name <- normalize_comment_name(name)

  parameterization <- NULL

  if (!is.null(raw) && nzchar(raw)) {
    parts <- extract_raw_sigma_parts(raw)
    if (is.null(name)) name <- parts$name
    parameterization <- map_parameterization(parts$parameterization, "SIGMA")
  }

  create_comment_with_sources(
    SigmaComment,
    sigma_fields(),
    mod_path,
    nonmem_name = nonmem_name,
    name = name,
    parameterization = parameterization
  )
}

# ==============================================================================
# Type1 comment parsing
# ==============================================================================

#' @noRd
parse_type1_comments <- function(
  param_names,
  parsed_comments,
  raw_comments,
  mod_path
) {
  nonmem_names <- names(param_names)

  # First pass: parse thetas to collect known theta names
  theta_names <- nonmem_names[grepl("^THETA", nonmem_names)]
  theta_comments <- lapply(theta_names, function(nonmem_name) {
    name <- param_names[[nonmem_name]]
    parsed <- parsed_comments[[nonmem_name]]
    raw <- raw_comments[[nonmem_name]]
    parse_type1_theta_comment(nonmem_name, name, parsed, raw, mod_path)
  })
  names(theta_comments) <- theta_names

  # Collect known theta names for context
  known_thetas <- vapply(
    theta_comments,
    function(c) c@name %||% "",
    character(1)
  )
  known_thetas <- known_thetas[nzchar(known_thetas)]

  # Second pass: parse omega/sigma with known_thetas context
  other_names <- nonmem_names[!grepl("^THETA", nonmem_names)]
  other_comments <- lapply(other_names, function(nonmem_name) {
    name <- param_names[[nonmem_name]]
    parsed <- parsed_comments[[nonmem_name]]
    raw <- raw_comments[[nonmem_name]]

    if (grepl("^OMEGA", nonmem_name)) {
      parse_type1_omega_comment(
        nonmem_name,
        name,
        parsed,
        raw,
        mod_path,
        known_thetas
      )
    } else if (grepl("^SIGMA", nonmem_name)) {
      parse_type1_sigma_comment(nonmem_name, name, parsed, raw, mod_path)
    } else {
      stop("Unknown parameter type: ", nonmem_name)
    }
  })
  names(other_comments) <- other_names

  # Combine and preserve original order
  comments <- c(theta_comments, other_comments)
  comments[nonmem_names]
}

#' @noRd
parse_type1_theta_comment <- function(
  nonmem_name,
  name,
  parsed,
  raw,
  mod_path
) {
  name <- normalize_comment_name(name)

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

  create_comment_with_sources(
    ThetaComment,
    theta_fields(),
    mod_path,
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
parse_type1_omega_comment <- function(
  nonmem_name,
  name,
  parsed,
  raw,
  mod_path,
  known_thetas = NULL
) {
  name <- normalize_comment_name(name)

  parameterization <- NULL
  associated_theta <- NULL

  # Parse name format: "OM1 (CL)" to extract associated_theta
  if (!is.null(name) && grepl("\\(.*\\)", name)) {
    theta_part <- gsub(".*\\((.+)\\).*", "\\1", name)
    # Use split_theta_reference for context-aware splitting
    associated_theta <- split_theta_reference(theta_part, known_thetas)
    name <- trimws(gsub("\\s*\\(.*\\)\\s*$", "", name))
  }

  # Try to extract from parsed comment
  if (!is.null(parsed) && !is.null(parsed$Type1)) {
    type1 <- parsed$Type1

    if (is.character(type1)) {
      # Type1$Unknown: raw string stored directly
      if (is.null(name)) {
        parsed_raw <- extract_raw_omega_parts(type1, known_thetas)
        name <- parsed_raw$name
        if (is.null(associated_theta))
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
      if (is.null(associated_theta)) associated_theta <- type1$theta_name
      if (is.null(parameterization))
        parameterization <- map_parameterization(
          type1$parameterization,
          "OMEGA"
        )
    }
  }

  # Fallback: extract from raw comment
  if (
    (is.null(name) || is.null(associated_theta)) &&
      !is.null(raw) &&
      nzchar(raw)
  ) {
    parsed_raw <- extract_raw_omega_parts(raw, known_thetas)
    if (is.null(name)) name <- parsed_raw$name
    if (is.null(associated_theta))
      associated_theta <- parsed_raw$associated_theta
    if (is.null(parameterization))
      parameterization <- map_parameterization(
        parsed_raw$parameterization,
        "OMEGA"
      )
  }

  create_comment_with_sources(
    OmegaComment,
    omega_fields(),
    mod_path,
    nonmem_name = nonmem_name,
    name = name,
    parameterization = parameterization,
    associated_theta = associated_theta
  )
}

#' @noRd
parse_type1_sigma_comment <- function(
  nonmem_name,
  name,
  parsed,
  raw,
  mod_path
) {
  name <- normalize_comment_name(name)

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

  create_comment_with_sources(
    SigmaComment,
    sigma_fields(),
    mod_path,
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
  # Colon after parameter identifier is optional
  raw <- gsub("^THETA\\(\\d+\\):?\\s*", "", raw, ignore.case = TRUE)
  raw <- gsub("^THETA\\d+:?\\s*", "", raw, ignore.case = TRUE)
  raw <- gsub("^OMEGA\\d+:?\\s*", "", raw, ignore.case = TRUE)
  raw <- gsub("^OMEGA\\(\\d+,\\d+\\):?\\s*", "", raw, ignore.case = TRUE)
  raw <- gsub("^SIGMA\\(\\d+\\):?\\s*", "", raw, ignore.case = TRUE)
  raw <- gsub("^SIGMA\\d+:?\\s*", "", raw, ignore.case = TRUE)
  raw <- gsub("^SIGMA\\(\\d+,\\d+\\):?\\s*", "", raw, ignore.case = TRUE)
  # Also handle bare number prefix like "1:", "1-", "1.", or "1 "
  raw <- gsub("^\\d+[-:.]?\\s*", "", raw)
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

  # Extract unit from parentheses or brackets
  # e.g., "CL (L/day)" or "CL [L/day]" -> unit="L/day"
  # e.g., "CL ([])" -> unit="[]", "CL [()]" -> unit="()"
  if (grepl("\\([^)]+\\)", raw)) {
    unit_match <- regmatches(raw, regexec("\\(([^)]+)\\)", raw))[[1]]
    if (length(unit_match) >= 2) {
      result$unit <- unit_match[2]
    }
    raw <- trimws(gsub("\\s*\\([^)]+\\)", "", raw))
  } else if (grepl("\\[[^\\]]+\\]", raw)) {
    unit_match <- regmatches(raw, regexec("\\[([^\\]]+)\\]", raw))[[1]]
    if (length(unit_match) >= 2) {
      result$unit <- unit_match[2]
    }
    raw <- trimws(gsub("\\s*\\[[^\\]]+\\]", "", raw))
  }

  # Find name (first word with letters)
  if (nzchar(raw)) {
    words <- strsplit(raw, "\\s+")[[1]]
    idx <- find_first_name_idx(words)
    if (!is.na(idx)) {
      # Strip trailing punctuation (comma, period, etc.)
      result$name <- gsub("[,.:;]+$", "", words[idx])
    }
  }

  result
}

#' Split theta reference into associated thetas
#'
#' Splits on separators unless the string matches a known theta name (case-insensitive).
#'
#' @param theta_ref Character string of the theta reference
#' @param known_thetas Character vector of known theta names for context
#' @return Character vector of associated theta names
#' @noRd
split_theta_reference <- function(theta_ref, known_thetas = NULL) {
  if (is.null(theta_ref) || !nzchar(theta_ref)) {
    return(NULL)
  }

  # Check if it matches a known theta (case-insensitive)
  if (!is.null(known_thetas) && length(known_thetas) > 0) {
    if (tolower(theta_ref) %in% tolower(known_thetas)) {
      return(theta_ref)
    }
  }

  # Otherwise split on separators
  if (grepl("[-/:,]", theta_ref)) {
    parts <- strsplit(theta_ref, "[-/:,]")[[1]]
    return(trimws(parts))
  }

  theta_ref
}

#' Extract components from raw omega comment string
#'
#' Parses comments like "OM1 CL", "OM1 CL :EXP", "OMEGA1: CL ; exp", or "OM2,1 CL-VC".
#' Builds composite names (e.g., "IIV CL" -> "IIV-CL") and extracts associated thetas.
#'
#' @param raw Character string of the raw comment
#' @param known_thetas Character vector of known theta names for context-aware splitting
#' @return Named list with name, associated_theta (character vector), and parameterization
#' @noRd
extract_raw_omega_parts <- function(raw, known_thetas = NULL) {
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

  # Split remaining into words and find first word with letters

  words <- strsplit(raw, "\\s+")[[1]]
  idx <- find_first_name_idx(words)

  if (is.na(idx)) {
    return(result)
  }

  first_word <- words[idx]
  prefix <- NULL
  theta_ref <- NULL

  # Check if first word already contains a hyphen (e.g., "IIV-CL", "Corr-CL-V")
  if (grepl("-", first_word)) {
    # Split on first hyphen only to get prefix
    hyphen_pos <- regexpr("-", first_word)
    prefix <- substr(first_word, 1, hyphen_pos - 1)
    theta_ref <- substr(first_word, hyphen_pos + 1, nchar(first_word))
  } else {
    # First word is the prefix, look for theta reference in subsequent words
    prefix <- first_word

    # Find theta reference, skipping linking words like "on", "for"
    linking_words <- c("on", "for", "of")
    theta_idx <- idx + 1
    while (
      theta_idx <= length(words) &&
        tolower(words[theta_idx]) %in% linking_words
    ) {
      theta_idx <- theta_idx + 1
    }

    if (theta_idx <= length(words)) {
      theta_ref <- words[theta_idx]
    }
  }

  # Store prefix as name, theta reference separately in associated_theta
  result$name <- prefix
  if (!is.null(theta_ref) && nzchar(theta_ref)) {
    result$associated_theta <- split_theta_reference(theta_ref, known_thetas)
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

  # Check if this is a numbered description (e.g., "1. Proportional error...", "1: Proportional error")
  # These are descriptions, not names - return NULL for name
  if (grepl("^\\d+(\\.|:)?\\s", raw)) {
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
