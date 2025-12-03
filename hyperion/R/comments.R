VALID_PARAMETERIZATIONS <- c(
  "Log",
  "Exp",
  "Add",
  "Prop",
  "Stdev",
  "Corr",
  "OmitTbl",
  "Var"
)

#' @noRd
ParameterComment <- S7::new_class(
  "ParameterComment",
  properties = list(
    original_name = S7::new_property(
      S7::class_character,
      validator = function(value) {
        if (length(value) != 1 || is.na(value) || trimws(value) == "") {
          "must be a non-empty string"
        }
      }
    )
  )
)

#' Type1 parameter comment class
#'
#' Represents Type1 format comments which include parameter name, optional unit,
#' optional parameterization, and optional associated theta reference.
#'
#' Inherits from `ParameterComment`.
#'
#' @param original_name Character. The NONMEM parameter name (inherited)
#' @param name Character or NULL. The user-defined parameter name (e.g., "TVCL", "OM1")
#' @param display Character or NULL. Display name for the parameter (e.g., "CL", "IIV-CL")
#' @param description Character or NULL. Description of the parameter (e.g., "Clearance")
#' @param unit Character or NULL. Unit of measurement (e.g., "L/hr")
#' @param parameterization Character or NULL. Transformation type. Valid values:
#'   "Log", "Exp", "Add", "Prop", "Stdev", "Corr", "OmitTbl", "Var"
#' @param associated_theta Character or NULL. For omega/sigma, the related theta name
#'
#' @export
Type1Comment <- S7::new_class(
  "Type1Comment",
  parent = ParameterComment,
  properties = list(
    name = S7::new_property(NULL | S7::class_character, default = NULL),
    display = S7::new_property(NULL | S7::class_character, default = NULL),
    description = S7::new_property(NULL | S7::class_character, default = NULL),
    unit = S7::new_property(NULL | S7::class_character, default = NULL),
    parameterization = S7::new_property(
      NULL | S7::class_character,
      default = NULL,
      setter = function(self, value) {
        if (!is.null(value)) {
          if (length(value) != 1 || is.na(value)) {
            stop("@parameterization must be a single non-NA string or NULL")
          }
          # Case-insensitive matching
          matched <- match(tolower(value), tolower(VALID_PARAMETERIZATIONS))
          if (is.na(matched)) {
            stop(paste0(
              "@parameterization must be one of: ",
              paste(VALID_PARAMETERIZATIONS, collapse = ", ")
            ))
          }
          # Normalize to canonical case
          value <- VALID_PARAMETERIZATIONS[matched]
        }
        self@parameterization <- value
        self
      }
    ),
    associated_theta = S7::new_property(
      NULL | S7::class_character,
      default = NULL
    )
  )
)

#' Extract all parameter comments from a model as S7 objects
#'
#' @param mod A hyperion_nonmem_model object or path to a control stream (.mod or .ctl)
#' @param lookup_path Optional path to a yaml lookup file. If provided, fills
#'   NULL fields (display, description, unit, parameterization) from the lookup.
#' @return Named list of ParameterComment objects, keyed by original_name
#'   (e.g., list(THETA1 = ..., "OMEGA(1,1)" = ...))
#' @export
comments_from_model <- function(mod, lookup_path = NULL) {
  if (is.character(mod) && length(mod) == 1) {
    mod <- read_model(mod)
  }

  if (!inherits(mod, "hyperion_nonmem_model")) {
    stop(
      "mod must be a hyperion_nonmem_model object or path to a control stream (.mod or .ctl)"
    )
  }

  param_names <- get_model_parameter_names(mod)
  parsed_comments <- extract_parsed_comments(mod)
  comments <- comments_from_hybrid(param_names, parsed_comments)

  if (!is.null(lookup_path)) {
    for (name in names(comments)) {
      comments[[name]] <- apply_lookup_defaults(comments[[name]], lookup_path)
    }
  }

  comments
}

#' @noRd
extract_parsed_comments <- function(mod) {
  comments <- list()

  for (i in seq_along(mod$theta_parameters)) {
    old_name <- paste0("THETA", i)
    comments[[old_name]] <- mod$theta_parameters[[i]]$parsed_comment
  }

  comments <- extract_block_comments(comments, mod$omega_blocks, "OMEGA")
  comments <- extract_block_comments(comments, mod$sigma_blocks, "SIGMA")

  comments
}

#' @noRd
extract_block_comments <- function(comments, blocks, prefix) {
  row <- 1

  for (block in blocks) {
    struct <- block$structure

    # Handle structure as string "Diagonal" or list with named element
    is_diagonal <- identical(struct, "Diagonal") || !is.null(struct$Diagonal)
    is_block <- is.list(struct) && !is.null(struct$Block)
    is_block_same <- is.list(struct) && !is.null(struct$BlockSame)

    if (is_diagonal) {
      for (param in block$parameters) {
        old_name <- sprintf("%s(%d,%d)", prefix, row, row)
        comments[[old_name]] <- param$parsed_comment
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
          comments[[old_name]] <- block$parameters[[param_idx]]$parsed_comment
          param_idx <- param_idx + 1
        }
      }
      row <- start_row + block_size
    } else if (is_block_same) {
      block_size <- struct$BlockSame$size
      row <- row + block_size
    }
  }

  comments
}

#' @noRd
comments_from_hybrid <- function(param_names, parsed_comments) {
  comment_type <- get_comment_type()

  if (is.null(comment_type)) {
    stop("comment_type not set in pharos.toml")
  }

  factory_fn <- switch(
    comment_type,
    "type1" = type1_comment_from_hybrid,
    stop("Unknown comment type: ", comment_type)
  )

  old_names <- names(param_names)
  comments <- lapply(old_names, function(old_name) {
    new_name <- param_names[[old_name]]
    parsed <- parsed_comments[[old_name]]
    factory_fn(old_name, new_name, parsed)
  })
  names(comments) <- old_names
  comments
}

#' @noRd
type1_comment_from_hybrid <- function(old_name, new_name, parsed) {
  name <- NULL
  unit <- NULL
  parameterization <- NULL
  associated_theta <- NULL

  if (!is.null(new_name) && nzchar(new_name)) {
    if (grepl("\\(.*\\)", new_name)) {
      associated_theta <- gsub(".*\\((.+)\\).*", "\\1", new_name)
      name <- trimws(gsub("\\s*\\(.*\\)\\s*$", "", new_name))
    } else {
      name <- new_name
    }
  }

  if (!is.null(parsed) && !is.null(parsed$Type1)) {
    type1 <- parsed$Type1

    if (!is.null(type1$WithUnit)) {
      if (is.null(name)) name <- type1$WithUnit$parameter
      unit <- type1$WithUnit$unit
      parameterization <- type1$WithUnit$parametrization
    } else {
      if (is.null(name)) name <- type1$name
      if (is.null(associated_theta)) associated_theta <- type1$theta_name
      parameterization <- type1$parameterization
    }
  }

  Type1Comment(
    original_name = old_name,
    name = name,
    unit = unit,
    parameterization = parameterization,
    associated_theta = associated_theta
  )
}

#' Apply lookup defaults to a parameter comment
#'
#' Fills NULL fields (display, description, unit, parameterization) from a
#' lookup yaml file. Matches the comment's `name` field against yaml keys.
#'
#' @param comment A Type1Comment object
#' @param lookup_path Path to a yaml lookup file
#' @return The modified comment object
#' @export
apply_lookup_defaults <- function(comment, lookup_path) {
  if (!S7::S7_inherits(comment, Type1Comment)) {
    stop("comment must be a Type1Comment object")
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

  if (is.null(comment@unit) && !is.null(entry$unit)) {
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
