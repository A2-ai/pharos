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
    "LOGADD" = "LogAddErr",
    "LOGADDERR" = "LogAddErr",
    "LOGERR" = "LogAddErr",
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

#' Helper to create a tracked property with source tracking
#' @param field_name The name of the field for source tracking
#' @param valid_values Optional vector of valid values. If provided, values are
#'   matched case-insensitively and normalized to the canonical form.
#' @noRd
make_tracked_property <- function(field_name, valid_values = NULL) {
  S7::new_property(
    NULL | S7::class_character,
    default = NULL,
    setter = function(self, value) {
      if (!is.null(value) && !is.null(valid_values)) {
        if (length(value) != 1 || is.na(value)) {
          rlang::abort(paste0(
            "@",
            field_name,
            " must be a single non-NA string or NULL"
          ))
        }
        matched <- match(tolower(value), tolower(valid_values))
        if (is.na(matched)) {
          rlang::abort(paste0(
            "@",
            field_name,
            " must be one of: ",
            paste(valid_values, collapse = ", "),
            ". Got: '",
            value,
            "'"
          ))
        }
        value <- valid_values[matched]
      }
      S7::prop(self, field_name) <- value
      sources <- attr(self, "sources")
      if (!is.null(sources)) {
        sources[[field_name]] <- "user supplied"
        attr(self, "sources") <- sources
      }
      self
    }
  )
}

#' Normalize omega associated_theta values against theta names
#'
#' If no exact match is found, tries matching by stripping trailing "/...".
#' Only applies when the base name maps unambiguously to a single theta name.
#'
#' @param assoc Character vector of associated theta names
#' @param theta_names Character vector of theta names
#' @return Character vector of normalized associated theta names
#' @noRd
normalize_associated_theta <- function(assoc, theta_names) {
  if (length(theta_names) == 0 || length(assoc) == 0) {
    return(assoc)
  }

  exact_lookup <- stats::setNames(theta_names, tolower(theta_names))

  base_names <- sub("/.*$", "", theta_names)
  base_lc <- tolower(base_names)
  base_map <- list()
  for (i in seq_along(theta_names)) {
    base_map[[base_lc[i]]] <- c(base_map[[base_lc[i]]], theta_names[i])
  }
  base_lookup <- vapply(
    base_map,
    function(vals) {
      if (length(unique(vals)) == 1) unique(vals) else NA_character_
    },
    character(1)
  )
  base_lookup <- base_lookup[!is.na(base_lookup)]

  vapply(
    assoc,
    function(theta) {
      key <- tolower(theta)
      if (key %in% names(exact_lookup)) {
        exact_lookup[[key]]
      } else if (key %in% names(base_lookup)) {
        base_lookup[[key]]
      } else {
        theta
      }
    },
    character(1)
  )
}

#' Rename duplicate omega names by appending associated_theta
#'
#' When multiple omega comments share the same name, renames ALL of them to
#' `{name}-{associated_theta}` to ensure uniqueness.
#'
#' @param omega List of OmegaComment objects
#' @return Modified list with duplicate names renamed
#' @noRd
rename_duplicate_omega_names <- function(omega) {
  if (length(omega) == 0) {
    return(omega)
  }

  omega_names <- vapply(
    omega,
    function(c) if (is.null(c@name)) NA_character_ else c@name,
    character(1)
  )

  # Find names that appear more than once (excluding NA)
  name_counts <- table(omega_names[!is.na(omega_names)])
  dup_names <- names(name_counts[name_counts > 1])

  if (length(dup_names) == 0) {
    return(omega)
  }

  lapply(omega, function(comment) {
    if (!is.null(comment@name) && comment@name %in% dup_names) {
      assoc <- comment@associated_theta
      if (!is.null(assoc) && length(assoc) == 1 && nzchar(assoc)) {
        new_name <- paste0(comment@name, "-", assoc)
        comment@name <- new_name
        sources <- attr(comment, "sources") %||% list()
        name_source <- sources[["name"]] %||% "default"
        sources[["name"]] <- paste0("renamed from ", name_source)
        attr(comment, "sources") <- sources
      }
    }
    comment
  })
}

#' Theta parameter comment class
#'
#' Represents parsed comments for THETA parameters.
#'
#' @param nonmem_name Character. The NONMEM parameter name (e.g., "THETA1").
#' @param name Character or NULL. User-defined parameter name (e.g., "CL", "V").
#' @param display Character or NULL. Display name for tables/output.
#' @param description Character or NULL. Description of the parameter.
#' @param unit Character or NULL. Unit of measurement (e.g., "L/hr").
#' @param parameterization Character or NULL. Transformation type.
#'
#' @section Properties:
#' \describe{
#'   \item{nonmem_name}{The NONMEM parameter identifier.}
#'   \item{name}{User-friendly name parsed from comments.}
#'   \item{display}{Display name for tables. Falls back to `name` if NULL.}
#'   \item{description}{Longer description of what the parameter represents.}
#'   \item{unit}{Unit of measurement for the parameter value.}
#'   \item{parameterization}{Transformation type. Valid values:
#'     "LogNormal", "Logit", "AddErr", "LogAddErr", "Proportional", "Identity".}
#' }
#'
#' @export
ThetaComment <- S7::new_class(
  "ThetaComment",
  parent = ParameterComment,
  properties = list(
    name = make_tracked_property("name"),
    display = make_tracked_property("display"),
    description = make_tracked_property("description"),
    unit = make_tracked_property("unit"),
    parameterization = make_tracked_property(
      "parameterization",
      valid_parameterizations()
    )
  )
)

#' Omega parameter comment class
#'
#' Represents parsed comments for OMEGA parameters.
#'
#' @param nonmem_name Character. The NONMEM parameter name (e.g., "OMEGA(1,1)").
#' @param name Character or NULL. User-defined parameter name (e.g., "IIV-CL").
#' @param display Character or NULL. Display name for tables/output.
#' @param description Character or NULL. Description of the parameter.
#' @param parameterization Character or NULL. Transformation type.
#' @param associated_theta Character vector or NULL. Related theta name(s).
#'
#' @section Properties:
#' \describe{
#'   \item{nonmem_name}{The NONMEM parameter identifier.}
#'   \item{name}{User-friendly name parsed from comments.}
#'   \item{display}{Display name for tables. Falls back to `name` if NULL.}
#'   \item{description}{Longer description of what the parameter represents.}
#'   \item{parameterization}{Transformation type. Valid values:
#'     "LogNormal", "Logit", "AddErr", "LogAddErr", "Proportional", "Identity".}
#'   \item{associated_theta}{Related theta parameter(s). For diagonal elements,
#'     typically a single name (e.g., "CL"). For off-diagonal (covariance),
#'     multiple names (e.g., c("CL", "V")).}
#' }
#'
#' @export
OmegaComment <- S7::new_class(
  "OmegaComment",
  parent = ParameterComment,
  properties = list(
    name = make_tracked_property("name"),
    display = make_tracked_property("display"),
    description = make_tracked_property("description"),
    parameterization = make_tracked_property(
      "parameterization",
      valid_parameterizations()
    ),
    associated_theta = make_tracked_property("associated_theta")
  )
)

#' Sigma parameter comment class
#'
#' Represents parsed comments for SIGMA parameters.
#'
#' @param nonmem_name Character. The NONMEM parameter name (e.g., "SIGMA(1,1)").
#' @param name Character or NULL. User-defined parameter name (e.g., "PropErr").
#' @param unit Character or NULL. Unit of measurement (e.g., "ng/mL").
#' @param display Character or NULL. Display name for tables/output.
#' @param description Character or NULL. Description of the parameter.
#' @param parameterization Character or NULL. Transformation type.
#'
#' @section Properties:
#' \describe{
#'   \item{nonmem_name}{The NONMEM parameter identifier.}
#'   \item{name}{User-friendly name parsed from comments.}
#'   \item{display}{Display name for tables. Falls back to `name` if NULL.}
#'   \item{description}{Longer description of what the parameter represents.}
#'   \item{parameterization}{Transformation type. Valid values:
#'     "LogNormal", "Logit", "AddErr", "LogAddErr", "Proportional", "Identity".}
#' }
#'
#' @export
SigmaComment <- S7::new_class(
  "SigmaComment",
  parent = ParameterComment,
  properties = list(
    name = make_tracked_property("name"),
    display = make_tracked_property("display"),
    description = make_tracked_property("description"),
    unit = make_tracked_property("unit"),
    parameterization = make_tracked_property(
      "parameterization",
      valid_parameterizations()
    )
  )
)

#' Model comments container with cross-validation
#'
#' Holds all parameter comments for a model organized by parameter type
#' (theta, omega, sigma) and validates cross-references between them.
#'
#' @param theta Named list of ThetaComment objects for THETA parameters.
#' @param omega Named list of OmegaComment objects for OMEGA parameters.
#' @param sigma Named list of SigmaComment objects for SIGMA parameters.
#'
#' @section Properties:
#' \describe{
#'   \item{theta}{Named list of ThetaComment objects, keyed by NONMEM name.}
#'   \item{omega}{Named list of OmegaComment objects, keyed by NONMEM name.}
#'   \item{sigma}{Named list of SigmaComment objects, keyed by NONMEM name.}
#' }
#'
#' @section Comment Style Guide:
#' `ModelComments` is populated by parsing NONMEM comments from `$THETA`,
#' `$OMEGA`, and `$SIGMA` blocks.
#'
#' For accepted raw comment formats and examples, see
#' [get_model_parameter_info()] ("Raw Comment Formats").
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

    # Type check each slot
    slot_classes <- list(
      theta = ThetaComment,
      omega = OmegaComment,
      sigma = SigmaComment
    )
    for (slot in names(slot_classes)) {
      comments <- S7::prop(self, slot)
      expected_class <- slot_classes[[slot]]
      class_name <- paste0(
        toupper(substr(slot, 1, 1)),
        substr(slot, 2, nchar(slot))
      )
      for (name in names(comments)) {
        if (!S7::S7_inherits(comments[[name]], expected_class)) {
          errors <- c(
            errors,
            sprintf(
              "%s$%s must be a %sComment object. Got: %s",
              slot,
              name,
              class_name,
              class(comments[[name]])[1]
            )
          )
        }
      }
    }

    # Helper to extract user names from comments
    extract_names <- function(comments) {
      if (length(comments) == 0) {
        return(character())
      }
      names_list <- vapply(
        comments,
        function(c) {
          if (!S7::S7_inherits(c, ParameterComment)) {
            return(NA_character_)
          }
          if (is.null(c@name)) NA_character_ else c@name
        },
        character(1)
      )
      names_list[!is.na(names_list)]
    }

    # Validate omega associated_theta references
    theta_names <- extract_names(self@theta)
    omega_comments <- self@omega
    for (omega_name in names(omega_comments)) {
      comment <- omega_comments[[omega_name]]
      if (!is.null(comment@associated_theta)) {
        assoc <- comment@associated_theta
        sources <- attr(comment, "sources") %||% list()
        assoc_source <- sources[["associated_theta"]] %||% "default"
        assoc_norm <- normalize_associated_theta(assoc, theta_names)
        # Validate against normalized associated_theta without mutating state.
        missing <- setdiff(assoc_norm, theta_names)
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

    # Check for duplicate names within each slot
    # For omega, check name + associated_theta uniqueness (not just name)
    for (slot in c("theta", "sigma")) {
      names_list <- extract_names(S7::prop(self, slot))
      dups <- names_list[duplicated(names_list)]
      if (length(dups) > 0) {
        errors <- c(
          errors,
          sprintf(
            "Duplicate names in %s: %s",
            slot,
            paste(unique(dups), collapse = ", ")
          )
        )
      }
    }

    # For omega, uniqueness is name + associated_theta
    omega_comments <- self@omega
    if (length(omega_comments) > 0) {
      omega_keys <- vapply(
        omega_comments,
        function(cmt) {
          if (is.null(cmt@name)) {
            return(NA_character_)
          }
          theta_str <- if (!is.null(cmt@associated_theta)) {
            paste(cmt@associated_theta, collapse = "-")
          } else {
            ""
          }
          paste(cmt@name, theta_str, sep = "|")
        },
        character(1)
      )
      omega_keys <- omega_keys[!is.na(omega_keys)]
      dups <- omega_keys[duplicated(omega_keys)]
      if (length(dups) > 0) {
        errors <- c(
          errors,
          sprintf(
            "Duplicate name + associated_theta in omega: %s",
            paste(unique(dups), collapse = ", ")
          )
        )
      }
    }

    if (length(errors) > 0) {
      return(paste(errors, collapse = "\n"))
    }
    NULL
  },
  constructor = function(theta = list(), omega = list(), sigma = list()) {
    if (length(theta) > 0 && length(omega) > 0) {
      theta_names <- vapply(
        theta,
        function(c) if (is.null(c@name)) NA_character_ else c@name,
        character(1)
      )
      theta_names <- theta_names[!is.na(theta_names)]
      if (length(theta_names) > 0) {
        omega <- lapply(omega, function(comment) {
          if (!is.null(comment@associated_theta)) {
            assoc <- comment@associated_theta
            sources <- attr(comment, "sources") %||% list()
            assoc_source <- sources[["associated_theta"]] %||% "default"
            assoc_norm <- normalize_associated_theta(assoc, theta_names)
            assoc_norm <- unname(assoc_norm)
            if (!identical(unname(assoc), assoc_norm)) {
              comment@associated_theta <- assoc_norm
              sources[["associated_theta"]] <- paste0(
                "normalized from ",
                assoc_source
              )
              attr(comment, "sources") <- sources
            }
          }
          comment
        })
      }
    }

    omega <- rename_duplicate_omega_names(omega)

    S7::new_object(
      S7::S7_object(),
      theta = theta,
      omega = omega,
      sigma = sigma
    )
  }
)

#' Find a parameter in ModelComments by name
#' @noRd
find_parameter <- function(info, parameter, kind = NULL) {
  if (!is.null(kind)) {
    kind <- toupper(kind)
    if (!kind %in% c("THETA", "OMEGA", "SIGMA")) {
      rlang::abort("kind must be one of: THETA, OMEGA, SIGMA")
    }
  }

  matches <- list()
  for (slot in c("theta", "omega", "sigma")) {
    if (!is.null(kind) && toupper(slot) != kind) {
      next
    }
    comments <- S7::prop(info, slot)

    # 1. Direct NONMEM name match (list key)
    if (parameter %in% names(comments)) {
      matches <- c(
        matches,
        list(list(slot = slot, key = parameter, obj = comments[[parameter]]))
      )
      next
    }

    # 2. Match by @name property
    for (key in names(comments)) {
      if (identical(comments[[key]]@name, parameter)) {
        matches <- c(
          matches,
          list(list(slot = slot, key = key, obj = comments[[key]]))
        )
      }
    }

    # 3. Match by @display property
    for (key in names(comments)) {
      if (identical(comments[[key]]@display, parameter)) {
        matches <- c(
          matches,
          list(list(slot = slot, key = key, obj = comments[[key]]))
        )
      }
    }
  }

  if (length(matches) == 0) {
    return(NULL)
  }
  if (length(matches) > 1) {
    rlang::abort(paste0(
      "Ambiguous parameter name '",
      parameter,
      "'. Provide kind."
    ))
  }
  matches[[1]]
}

#' Update parameter info in a ModelComments object
#'
#' @param info A ModelComments object
#' @param parameter The parameter to update. Can be a NONMEM name (e.g., "THETA1",
#'   "OMEGA(1,1)") or a custom name/display value.
#' @param kind Optional character. Filter by parameter kind ("THETA", "OMEGA",
#'   or "SIGMA"). Required if the name is ambiguous across kinds.
#' @param name User-defined parameter name (e.g., "CL", "IIV-CL")
#' @param display Display name for tables/output
#' @param description Description of the parameter
#' @param unit Unit of measurement. Applies to THETA and SIGMA parameters.
#' @param parameterization Transformation type. Valid values: "LogNormal", "Logit",
#'   "AddErr", "LogAddErr", "Proportional", "Identity".
#' @param associated_theta Related theta name(s). Only applies to OMEGA parameters.
#' @return The updated ModelComments object
#' @export
update_param_info <- function(
  info,
  parameter,
  kind = NULL,
  name = NULL,
  display = NULL,
  description = NULL,
  unit = NULL,
  parameterization = NULL,
  associated_theta = NULL
) {
  found <- find_parameter(info, parameter, kind = kind)

  if (is.null(found)) {
    rlang::abort(paste0(
      "Parameter '",
      parameter,
      "' not found in ModelComments"
    ))
  }

  slot <- found$slot
  key <- found$key
  param_obj <- found$obj

  # Common properties (all types)
  if (!is.null(name)) {
    param_obj@name <- name
  }
  if (!is.null(display)) {
    param_obj@display <- display
  }
  if (!is.null(description)) {
    param_obj@description <- description
  }
  if (!is.null(parameterization)) {
    param_obj@parameterization <- parameterization
  }

  # THETA/SIGMA: unit
  if (!is.null(unit)) {
    if (!slot %in% c("theta", "sigma")) {
      rlang::warn("'unit' only applies to THETA and SIGMA parameters, ignoring")
    } else {
      param_obj@unit <- unit
    }
  }

  # OMEGA-only: associated_theta
  if (!is.null(associated_theta)) {
    if (slot != "omega") {
      rlang::warn(
        "'associated_theta' only applies to OMEGA parameters, ignoring"
      )
    } else {
      param_obj@associated_theta <- associated_theta
    }
  }

  # Update the slot
  comments <- S7::prop(info, slot)
  comments[[key]] <- param_obj
  S7::prop(info, slot) <- comments

  info
}
