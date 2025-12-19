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

#' Helper to create parameterization property with validation
#' @noRd
make_parameterization_property <- function() {
  S7::new_property(
    NULL | S7::class_character,
    default = NULL,
    setter = function(self, value) {
      # Allow NULL (means not set, will default to Identity at usage)
      if (is.null(value)) {
        self@parameterization <- NULL
        return(self)
      }
      if (length(value) != 1 || is.na(value)) {
        stop("@parameterization must be a single non-NA string or NULL")
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
