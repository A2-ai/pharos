#' Format omega display name, avoiding duplicate theta info
#'
#' Builds a display name for omega parameters by appending associated theta
#' information, but only if that information isn't already present in the name.
#' This prevents duplication like "IIV-CL (CL)" when the omega was already
#' renamed to include the theta.
#'
#' @param name The omega parameter name (e.g., "IIV-CL" or "IIV")
#' @param associated_theta Character vector of associated theta names
#' @param theta_labels Optional named vector mapping theta names to display
#'   labels. If provided, uses labels for the suffix; otherwise uses theta names.
#'
#' @return The formatted display name with theta info appended only if missing
#'
#' @examples
#' # Theta already in name - no duplication
#' format_omega_display_name("IIV-CL", "CL")
#' # Returns: "IIV-CL"
#'
#' # Theta not in name - appends it
#' format_omega_display_name("IIV", "CL")
#' # Returns: "IIV CL"
#'
#' # Multiple thetas
#' format_omega_display_name("IIV", c("CL", "V"))
#' # Returns: "IIV CL, V"
#'
#' # With custom labels
#' format_omega_display_name("IIV", "CL", c(CL = "Clearance"))
#' # Returns: "IIV Clearance"
#'
#' @export
format_omega_display_name <- function(
  name,
  associated_theta,
  theta_labels = NULL
) {
  if (is.null(associated_theta) || length(associated_theta) == 0) {
    return(name)
  }

  # Determine what labels to use for checking and appending
  if (!is.null(theta_labels)) {
    labels_to_use <- vapply(
      associated_theta,
      function(theta) {
        if (theta %in% names(theta_labels)) {
          theta_labels[[theta]]
        } else {
          theta
        }
      },
      character(1)
    )
  } else {
    labels_to_use <- associated_theta
  }

  # Check which thetas are already present in the name
  # Use word boundary pattern to avoid matching "V" in "COV"
  theta_already_present <- vapply(
    seq_along(associated_theta),
    function(i) {
      # Build pattern that matches theta as a distinct component
      # (at word boundary or separated by common delimiters like -, /, space)
      safe_theta <- gsub(
        "([][{}()^$*+?.|\\\\])",
        "\\\\\\1",
        associated_theta[i]
      )
      theta_pattern <- paste0(
        "(^|[^A-Za-z0-9])",
        safe_theta,
        "($|[^A-Za-z0-9])"
      )
      if (grepl(theta_pattern, name)) {
        return(TRUE)
      }
      # Check if theta label is in target (with same boundary check)
      safe_label <- gsub(
        "([][{}()^$*+?.|\\\\])",
        "\\\\\\1",
        labels_to_use[i]
      )
      label_pattern <- paste0(
        "(^|[^A-Za-z0-9])",
        safe_label,
        "($|[^A-Za-z0-9])"
      )
      if (grepl(label_pattern, name)) {
        return(TRUE)
      }
      FALSE
    },
    logical(1)
  )

  # Only append missing thetas
  missing_labels <- labels_to_use[!theta_already_present]
  if (length(missing_labels) > 0) {
    theta_str <- paste(missing_labels, collapse = ", ")
    paste0(name, " ", theta_str)
  } else {
    name
  }
}

#' Convert comment list to data frame with values
#' @param comments Named list of comment objects
#' @param fields Character vector of field names to extract
#' @param value_resolver Function(comment, field) -> value or NULL
#' @return Data frame with parameter column and value columns
#' @noRd
comment_list_to_df <- function(comments, fields, value_resolver) {
  if (length(comments) == 0) {
    df <- data.frame(parameter = character(), stringsAsFactors = FALSE)
    for (f in fields) df[[f]] <- character()
    return(df)
  }

  rows <- lapply(names(comments), function(nm) {
    cmt <- comments[[nm]]
    row <- data.frame(parameter = nm, stringsAsFactors = FALSE)
    for (f in fields) {
      val <- value_resolver(cmt, f)
      if (is.null(val)) {
        row[[f]] <- NA_character_
      } else if (length(val) > 1) {
        row[[f]] <- paste(val, collapse = ", ")
      } else {
        row[[f]] <- val
      }
    }
    row
  })
  do.call(rbind, rows)
}

#' Build comment tables for theta/omega/sigma slots
#' @param comments_list Named list of comment lists
#' @param fields_list Named list of fields vectors
#' @param value_resolver Function(comment, field) -> value or NULL
#' @return Named list of data frames
#' @noRd
build_comment_tables <- function(comments_list, fields_list, value_resolver) {
  tables <- list()
  for (slot in names(comments_list)) {
    tables[[slot]] <- comment_list_to_df(
      comments_list[[slot]],
      fields_list[[slot]],
      value_resolver
    )
  }
  tables
}
