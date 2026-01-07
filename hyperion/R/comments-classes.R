VALID_PARAMETERIZATIONS <- c(
  "LogNormal",
  "Logit",
  "AddErr",
  "LogAddErr",
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
          stop("@", field_name, " must be a single non-NA string or NULL")
        }
        matched <- match(tolower(value), tolower(valid_values))
        if (is.na(matched)) {
          stop(
            "@",
            field_name,
            " must be one of: ",
            paste(valid_values, collapse = ", ")
          )
        }
        value <- valid_values[matched]
      }
      S7::prop(self, field_name) <- value
      sources <- attr(self, "sources")
      if (!is.null(sources)) {
        sources[[field_name]] <- "hard-coded"
        attr(self, "sources") <- sources
      }
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
    name = make_tracked_property("name"),
    display = make_tracked_property("display"),
    description = make_tracked_property("description"),
    unit = make_tracked_property("unit"),
    parameterization = make_tracked_property(
      "parameterization",
      VALID_PARAMETERIZATIONS
    )
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
    name = make_tracked_property("name"),
    display = make_tracked_property("display"),
    description = make_tracked_property("description"),
    parameterization = make_tracked_property(
      "parameterization",
      VALID_PARAMETERIZATIONS
    ),
    associated_theta = make_tracked_property("associated_theta")
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
    name = make_tracked_property("name"),
    display = make_tracked_property("display"),
    description = make_tracked_property("description"),
    parameterization = make_tracked_property(
      "parameterization",
      VALID_PARAMETERIZATIONS
    )
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
            sprintf("%s$%s must be a %sComment object", slot, name, class_name)
          )
        }
      }
    }

    # Helper to extract user names from comments
    extract_names <- function(comments) {
      if (length(comments) == 0) return(character())
      names_list <- vapply(
        comments,
        function(c) if (is.null(c@name)) NA_character_ else c@name,
        character(1)
      )
      names_list[!is.na(names_list)]
    }

    # Validate omega associated_theta references
    theta_names <- extract_names(self@theta)
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

    # Check for duplicate names within each slot
    for (slot in c("theta", "omega", "sigma")) {
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

    if (length(errors) > 0) {
      return(paste(errors, collapse = "\n"))
    }
    NULL
  }
)
